//! MLS group E2EE (RFC 9420) wrapper
//! 
//! Thin, production-oriented wrapper over openmls 0.8 implementing group
//! end-to-end encryption. The provider (crypto + randomness) comes from
//! `openmls_rust_crypto`; MLS state at rest lives in an in-memory
//! `openmls_memory_storage` storage which we serialize explicitly for
//! persistence (`MlsGroup` itself is not serde in openmls 0.8 — it is
//! reconstructed from storage via `MlsGroup::load`).
//! 
//! Member onboarding: `add_member` generates the new member's key package
//! and returns the serialized `Welcome`. The new member's signing keypair is
//! generated here; delivering its private half is out-of-band (seal it with
//! the DM ratchet or the group's export secret before transport).

#![allow(dead_code)] // wired by commands/mls.rs

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use openmls::{
    credentials::{BasicCredential, CredentialWithKey},
    framing::{MlsMessageBodyIn, MlsMessageIn, ProcessedMessageContent},
    group::{GroupId, MlsGroup, MlsGroupCreateConfig, MlsGroupJoinConfig, StagedWelcome},
    key_packages::{KeyPackage, KeyPackageIn},
    prelude::LeafNodeIndex,
    versions::ProtocolVersion,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_memory_storage::MemoryStorage;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::types::Ciphersuite;
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};
use tls_codec::{Deserialize as TlsDeserializeTrait, Serialize as TlsSerializeTrait};

use crate::crypto::random_bytes;
use crate::error::{VeilError, VeilResult};

pub struct MlsGroupSession {
    provider: OpenMlsRustCrypto,
    signer: SignatureKeyPair,
    credential_with_key: CredentialWithKey,
    group: MlsGroup,
    name: String,
}

/// At-rest form: group state is rebuilt via `MlsGroup::load` from the storage
/// snapshot; everything else serializes directly.
/// (`SignatureKeyPair` is not `Clone`, so serialization borrows it.)
#[derive(Serialize)]
struct MlsGroupSessionAtRestRef<'a> {
    credential_with_key: &'a CredentialWithKey,
    signer: &'a SignatureKeyPair,
    group_id: &'a GroupId,
    storage: Vec<(String, String)>,
    name: &'a str,
}

#[derive(Deserialize)]
struct MlsGroupSessionAtRest {
    credential_with_key: CredentialWithKey,
    signer: SignatureKeyPair,
    group_id: GroupId,
    storage: Vec<(String, String)>,
    name: String,
}

impl MlsGroupSession {
    /// Create a new MLS group with the caller as the only member
    pub fn create(ciphersuite: Ciphersuite, name: &str) -> VeilResult<Self> {
        let group_id = GroupId::from_slice(&random_bytes(32)?);
        Self::create_with_group_id(ciphersuite, name, group_id.as_slice())
    }

    /// Create a new MLS group pinned to a deterministic group id (channel id).
    pub fn create_with_group_id(ciphersuite: Ciphersuite, name: &str, group_id: &[u8]) -> VeilResult<Self> {
        let provider = OpenMlsRustCrypto::default();

        let signer = SignatureKeyPair::new(ciphersuite.signature_algorithm())
            .map_err(|_| VeilError::CryptoError)?;
        signer
            .store(provider.storage())
            .map_err(|_| VeilError::CryptoError)?;

        let credential_with_key = CredentialWithKey {
            credential: BasicCredential::new(name.as_bytes().to_vec()).into(),
            signature_key: signer.public().into(),
        };

        let group_id = GroupId::from_slice(group_id);
        let create_config = MlsGroupCreateConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();
        let group = MlsGroup::new_with_group_id(
            &provider,
            &signer,
            &create_config,
            group_id,
            credential_with_key.clone(),
        )
        .map_err(|_| VeilError::CryptoError)?;

        Ok(Self {
            provider,
            signer,
            credential_with_key,
            group,
            name: name.to_string(),
        })
    }

    /// New member creates their own key package (signer key stays with them).
    /// Returns (serialized KeyPackage, serialized SignerKey, dumped memory storage entries).
    pub fn create_key_package(ciphersuite: Ciphersuite, identity: &str) -> VeilResult<(Vec<u8>, Vec<u8>, Vec<(String, String)>)> {
        let provider = OpenMlsRustCrypto::default();
        let signer = SignatureKeyPair::new(ciphersuite.signature_algorithm())
            .map_err(|_| VeilError::CryptoError)?;
        signer
            .store(provider.storage())
            .map_err(|_| VeilError::CryptoError)?;
        let credential = CredentialWithKey {
            credential: BasicCredential::new(identity.as_bytes().to_vec()).into(),
            signature_key: signer.public().into(),
        };
        let key_package = KeyPackage::builder()
            .build(ciphersuite, &provider, &signer, credential)
            .map_err(|_| VeilError::CryptoError)?
            .key_package()
            .clone();
        let bytes = key_package
            .tls_serialize_detached()
            .map_err(|_| VeilError::SerializationError)?;
        // Signer özel anahtarı (gizli) — yeni üye saklar.
        let signer_bytes = signer
            .tls_serialize_detached()
            .map_err(|_| VeilError::SerializationError)?;
        let storage = dump_storage(provider.storage())?;
        Ok((bytes, signer_bytes, storage))
    }

    /// Owner: add a member from their serialized KeyPackage; returns the Welcome.
    pub fn add_key_package(&mut self, key_package_bytes: &[u8]) -> VeilResult<Vec<u8>> {
        let key_package = KeyPackageIn::tls_deserialize(&mut &key_package_bytes[..])
            .map_err(|_| VeilError::InvalidInput("Invalid key package".into()))?
            .validate(self.provider.crypto(), ProtocolVersion::default())
            .map_err(|_| VeilError::InvalidInput("Invalid key package".into()))?;
        let (_commit, welcome, _group_info) = self
            .group
            .add_members(&self.provider, &self.signer, &[key_package])
            .map_err(|_| VeilError::CryptoError)?;
        self.group
            .merge_pending_commit(&self.provider)
            .map_err(|_| VeilError::CryptoError)?;
        welcome
            .tls_serialize_detached()
            .map_err(|_| VeilError::SerializationError)
    }

    /// Member: join from a Welcome (their own signer key and key package storage must be pre-stored).
    pub fn join_from_welcome(
        welcome_bytes: &[u8],
        signer_bytes: &[u8],
        member_storage: &[(String, String)],
        name: &str,
    ) -> VeilResult<Self> {
        let provider = OpenMlsRustCrypto::default();
        restore_storage(provider.storage(), member_storage)?;
        let signer = SignatureKeyPair::tls_deserialize(&mut &signer_bytes[..])
            .map_err(|_| VeilError::CryptoError)?;
        signer
            .store(provider.storage())
            .map_err(|_| VeilError::CryptoError)?;

        let welcome = MlsMessageIn::tls_deserialize(&mut &welcome_bytes[..])
            .map_err(|_| VeilError::DecryptionError)?;
        let welcome = match welcome.extract() {
            MlsMessageBodyIn::Welcome(w) => w,
            _ => return Err(VeilError::DecryptionError),
        };
        let join_config = MlsGroupJoinConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();
        let staged = StagedWelcome::new_from_welcome(&provider, &join_config, welcome, None)
            .map_err(|_| VeilError::DecryptionError)?;
        let group = staged
            .into_group(&provider)
            .map_err(|_| VeilError::DecryptionError)?;

        let credential_with_key = CredentialWithKey {
            credential: BasicCredential::new(name.as_bytes().to_vec()).into(),
            signature_key: signer.public().into(),
        };
        Ok(Self {
            provider,
            signer,
            credential_with_key,
            group,
            name: name.to_string(),
        })
    }

    /// Group export secret — per-room media E2EE keys derive from this.
    pub fn export_secret(&self, label: &str) -> VeilResult<Vec<u8>> {
        self.group
            .export_secret(self.provider.crypto(), label, &[], 32)
            .map_err(|_| VeilError::CryptoError)
    }

    /// Add a member to the group. `credential_pub_key` is a hex-encoded
    /// identity for the new member. Returns the serialized `Welcome` message
    /// for the new member (deliver together with the generated signing key,
    /// see module docs).
    pub fn add_member(&mut self, credential_pub_key: &str) -> VeilResult<Vec<u8>> {
        let identity = hex::decode(credential_pub_key)
            .map_err(|_| VeilError::InvalidInput("Invalid credential hex".into()))?;

        let ciphersuite = self.group.ciphersuite();
        let member_signer = SignatureKeyPair::new(ciphersuite.signature_algorithm())
            .map_err(|_| VeilError::CryptoError)?;
        member_signer
            .store(self.provider.storage())
            .map_err(|_| VeilError::CryptoError)?;

        let member_credential = CredentialWithKey {
            credential: BasicCredential::new(identity).into(),
            signature_key: member_signer.public().into(),
        };
        let key_package = KeyPackage::builder()
            .build(ciphersuite, &self.provider, &member_signer, member_credential)
            .map_err(|_| VeilError::CryptoError)?
            .key_package()
            .clone();

        // openmls 0.8: add_members -> (commit, welcome, group_info)
        let (_commit, welcome, _group_info) = self
            .group
            .add_members(&self.provider, &self.signer, &[key_package])
            .map_err(|_| VeilError::CryptoError)?;
        self.group
            .merge_pending_commit(&self.provider)
            .map_err(|_| VeilError::CryptoError)?;

        welcome
            .tls_serialize_detached()
            .map_err(|_| VeilError::SerializationError)
    }

    /// Remove a member from the group by leaf index
    pub fn remove_member(&mut self, leaf_index: u32) -> VeilResult<()> {
        let (_commit, _welcome, _group_info) = self
            .group
            .remove_members(
                &self.provider,
                &self.signer,
                &[LeafNodeIndex::new(leaf_index)],
            )
            .map_err(|_| VeilError::CryptoError)?;
        self.group
            .merge_pending_commit(&self.provider)
            .map_err(|_| VeilError::CryptoError)?;
        Ok(())
    }

    /// Encrypt an application message for the group (serialized MLS message)
    pub fn encrypt_message(&mut self, plaintext: &[u8]) -> VeilResult<Vec<u8>> {
        let message = self
            .group
            .create_message(&self.provider, &self.signer, plaintext)
            .map_err(|_| VeilError::EncryptionError)?;
        message
            .tls_serialize_detached()
            .map_err(|_| VeilError::SerializationError)
    }

    /// Decrypt an incoming group message (serialized MLS message → plaintext)
    pub fn decrypt_message(&mut self, message: &[u8]) -> VeilResult<Vec<u8>> {
        let bytes = message.to_vec();
        let msg_in = MlsMessageIn::tls_deserialize(&mut bytes.as_slice())
            .map_err(|_| VeilError::DecryptionError)?;
        let protocol_message = msg_in
            .try_into_protocol_message()
            .map_err(|_| VeilError::DecryptionError)?;

        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.group.process_message(&self.provider, protocol_message)
        }));

        let processed = match res {
            Ok(Ok(p)) => p,
            _ => return Err(VeilError::DecryptionError),
        };

        match processed.into_content() {
            ProcessedMessageContent::ApplicationMessage(app) => Ok(app.into_bytes()),
            // Handshake/proposal messages carry no application plaintext
            _ => Err(VeilError::DecryptionError),
        }
    }

    /// Serialize the session for at-rest storage (encrypt the result with the
    /// DB key before persisting — this blob contains private key material).
    pub fn serialize(&self) -> VeilResult<Vec<u8>> {
        let at_rest = MlsGroupSessionAtRestRef {
            credential_with_key: &self.credential_with_key,
            signer: &self.signer,
            group_id: self.group.group_id(),
            storage: dump_storage(self.provider.storage())?,
            name: &self.name,
        };
        serde_json::to_vec(&at_rest).map_err(|_| VeilError::SerializationError)
    }

    /// Deserialize a session previously written by [`Self::serialize`]
    pub fn deserialize(bytes: &[u8]) -> VeilResult<Self> {
        let at_rest: MlsGroupSessionAtRest =
            serde_json::from_slice(bytes).map_err(|_| VeilError::SerializationError)?;

        let provider = OpenMlsRustCrypto::default();
        restore_storage(provider.storage(), &at_rest.storage)?;

        let group = MlsGroup::load(provider.storage(), &at_rest.group_id)
            .map_err(|_| VeilError::SerializationError)?
            .ok_or(VeilError::SerializationError)?;

        Ok(Self {
            provider,
            signer: at_rest.signer,
            credential_with_key: at_rest.credential_with_key,
            group,
            name: at_rest.name,
        })
    }

    /// Group id (hex) — useful for keying per-group state in the DB
    pub fn group_id(&self) -> String {
        hex::encode(self.group.group_id().as_slice())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

fn dump_storage(storage: &MemoryStorage) -> VeilResult<Vec<(String, String)>> {
    let values = storage
        .values
        .read()
        .map_err(|_| VeilError::SerializationError)?;
    let mut out = Vec::with_capacity(values.len());
    for (k, v) in values.iter() {
        out.push((B64.encode(k), B64.encode(v)));
    }
    Ok(out)
}

fn restore_storage(storage: &MemoryStorage, entries: &[(String, String)]) -> VeilResult<()> {
    let mut values = storage
        .values
        .write()
        .map_err(|_| VeilError::SerializationError)?;
    for (k, v) in entries {
        let key = B64.decode(k).map_err(|_| VeilError::SerializationError)?;
        let value = B64.decode(v).map_err(|_| VeilError::SerializationError)?;
        values.insert(key, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

    #[test]
    fn mls_group_creation_and_properties() {
        let session =
            MlsGroupSession::create(TEST_CIPHERSUITE, "alice-channel").expect("group creation should succeed");
        assert_eq!(session.name(), "alice-channel");
        assert_eq!(session.group_id().len(), 64, "group_id must be 32 bytes hex encoded");

        let custom_id = [0x42u8; 32];
        let pinned_session = MlsGroupSession::create_with_group_id(TEST_CIPHERSUITE, "pinned-channel", &custom_id)
            .expect("pinned group creation should succeed");
        assert_eq!(pinned_session.group_id(), hex::encode(custom_id));
        assert_eq!(pinned_session.name(), "pinned-channel");
    }

    #[test]
    fn mls_export_secret_is_deterministic() {
        let session = MlsGroupSession::create(TEST_CIPHERSUITE, "media-channel").expect("group creation");
        let secret1 = session.export_secret("veilanon-media-e2ee").expect("export secret 1");
        let secret2 = session.export_secret("veilanon-media-e2ee").expect("export secret 2");
        assert_eq!(secret1.len(), 32);
        assert_eq!(secret1, secret2, "export secret with same label must be deterministic");

        let secret_other = session.export_secret("different-label").expect("export secret other");
        assert_ne!(secret1, secret_other, "different export labels must produce distinct secrets");
    }

    #[test]
    fn mls_member_join_and_messaging() {
        // 1. Alice creates the MLS group
        let mut alice = MlsGroupSession::create(TEST_CIPHERSUITE, "alice").expect("create alice group");

        // 2. Bob creates a KeyPackage
        let (bob_kp_bytes, bob_signer_bytes, bob_storage) =
            MlsGroupSession::create_key_package(TEST_CIPHERSUITE, "bob").expect("create bob keypackage");
        assert!(!bob_kp_bytes.is_empty());
        assert!(!bob_signer_bytes.is_empty());

        // 3. Alice adds Bob's KeyPackage and produces a Welcome envelope
        let welcome_bytes = alice.add_key_package(&bob_kp_bytes).expect("alice add bob keypackage");
        assert!(!welcome_bytes.is_empty());

        // 4. Bob joins using the Welcome message and his signer private key
        let mut bob = MlsGroupSession::join_from_welcome(&welcome_bytes, &bob_signer_bytes, &bob_storage, "bob")
            .expect("bob join from welcome");
        assert_eq!(alice.group_id(), bob.group_id());

        // 5. Alice encrypts application message -> Bob decrypts
        let plaintext = b"Hello Bob! Welcome to VeilAnon MLS E2EE channel.";
        let encrypted_msg = alice.encrypt_message(plaintext).expect("alice encrypt message");
        let decrypted = bob.decrypt_message(&encrypted_msg).expect("bob decrypt message");
        assert_eq!(decrypted, plaintext);

        // 6. Export secret matches between Alice and Bob
        let alice_export = alice.export_secret("call-key-v1").expect("alice export");
        let bob_export = bob.export_secret("call-key-v1").expect("bob export");
        assert_eq!(alice_export, bob_export, "export secrets must match across all members");
    }

    #[test]
    fn mls_session_serialization_roundtrip() {
        let mut alice = MlsGroupSession::create(TEST_CIPHERSUITE, "persistent-channel").expect("create session");
        let group_id_orig = alice.group_id();

        // Add Bob so group state contains credentials and roster
        let (bob_kp, bob_signer, bob_storage) =
            MlsGroupSession::create_key_package(TEST_CIPHERSUITE, "bob").expect("bob kp");
        let welcome = alice.add_key_package(&bob_kp).expect("alice add bob");
        let mut bob = MlsGroupSession::join_from_welcome(&welcome, &bob_signer, &bob_storage, "bob").expect("bob join");

        // Serialize Alice's session
        let serialized = alice.serialize().expect("serialize alice session");
        assert!(!serialized.is_empty());

        // Deserialize Alice's session
        let mut loaded_alice = MlsGroupSession::deserialize(&serialized).expect("deserialize alice session");
        assert_eq!(loaded_alice.group_id(), group_id_orig);
        assert_eq!(loaded_alice.name(), "persistent-channel");

        // Alice encrypts from restored session -> Bob successfully decrypts
        let msg = loaded_alice
            .encrypt_message(b"Post-deserialization message")
            .expect("encrypt from restored");
        let decrypted = bob
            .decrypt_message(&msg)
            .expect("bob decrypt post-restore message");
        assert_eq!(decrypted, b"Post-deserialization message");
    }

    #[test]
    fn mls_add_invalid_key_package_fails() {
        let mut alice = MlsGroupSession::create(TEST_CIPHERSUITE, "alice").expect("create alice");
        let invalid_kp = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let res = alice.add_key_package(&invalid_kp);
        assert!(res.is_err(), "adding malformed key package must fail");
    }

    #[test]
    fn mls_join_invalid_welcome_fails() {
        let (_kp, bob_signer, bob_storage) =
            MlsGroupSession::create_key_package(TEST_CIPHERSUITE, "bob").expect("bob kp");
        let invalid_welcome = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let res = MlsGroupSession::join_from_welcome(&invalid_welcome, &bob_signer, &bob_storage, "bob");
        assert!(res.is_err(), "joining with invalid welcome must fail");
    }

    #[test]
    fn mls_decrypt_tampered_message_fails() {
        let mut alice = MlsGroupSession::create(TEST_CIPHERSUITE, "alice").expect("create alice");
        let (bob_kp, bob_signer, bob_storage) =
            MlsGroupSession::create_key_package(TEST_CIPHERSUITE, "bob").expect("bob kp");
        let welcome = alice.add_key_package(&bob_kp).expect("alice add");
        let mut bob = MlsGroupSession::join_from_welcome(&welcome, &bob_signer, &bob_storage, "bob").expect("bob join");

        let mut encrypted = alice.encrypt_message(b"Secret payload").expect("encrypt");
        let last_byte = encrypted.len() - 1;
        encrypted[last_byte] ^= 0xFF; // tamper with ciphertext

        let res = bob.decrypt_message(&encrypted);
        assert!(res.is_err(), "decrypting tampered message must return error");
    }
}
