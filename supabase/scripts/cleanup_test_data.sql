-- ============================================================================
-- veilanon — Test Data Cleanup Script
-- ONLY deletes data, NEVER drops tables or changes schema
-- Run this to clear all test/development data from Supabase
-- ============================================================================

-- Delete all messages (ciphertext only, no plaintext)
DELETE FROM public.messages;

-- Delete all friendships
DELETE FROM public.friendships;

-- Delete all channel memberships
DELETE FROM public.channel_members;

-- Delete all channels
DELETE FROM public.channels;

-- Delete all memberships (space members)
DELETE FROM public.memberships;

-- Delete all roles and role assignments
DELETE FROM public.role_members;
DELETE FROM public.roles;

-- Delete all invites
DELETE FROM public.invites;

-- Delete all bans
DELETE FROM public.bans;

-- Delete all spaces
DELETE FROM public.spaces;

-- Delete all devices (E2EE key registry)
DELETE FROM public.devices;

-- Delete all presence records
DELETE FROM public.presence;

-- Delete all users (GoTrue auth.users + public.users)
-- NOTE: This also deletes auth.users via cascade if enabled
DELETE FROM public.users;

-- Reset any sequences if they exist
-- (Supabase uses UUIDs so sequences are not typically used)

-- Verify cleanup
SELECT 
  (SELECT COUNT(*) FROM public.messages) as messages_count,
  (SELECT COUNT(*) FROM public.friendships) as friendships_count,
  (SELECT COUNT(*) FROM public.channels) as channels_count,
  (SELECT COUNT(*) FROM public.memberships) as memberships_count,
  (SELECT COUNT(*) FROM public.spaces) as spaces_count,
  (SELECT COUNT(*) FROM public.users) as users_count;
