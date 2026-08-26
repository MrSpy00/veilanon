$envFile = Get-Content ".env" | Where-Object { $_ -notmatch "^#" -and $_ -match "=" }
$supa = @{}
foreach ($l in $envFile) {
    $parts = $l -split "=", 2
    $supa[$parts[0]] = $parts[1]
}
$url = $supa["VEILANON_SUPABASE_URL"]
$srk = $supa["VEILANON_SUPABASE_SERVICE_ROLE_KEY"]
$headers = @{
    "apikey" = $srk
    "Authorization" = "Bearer $srk"
    "Content-Type" = "application/json"
}
$tables = @(
    "users","spaces","channels","messages","reactions","invites","devices",
    "friends","friendships","user_profiles","content_reports","server_members",
    "channel_members","mls_keypackages","mls_groups","mls_conversations",
    "dm_messages","dm_threads","voice_rooms","voice_room_members","user_blocks",
    "notifications","mls_pending_welcomes","message_reports","messages_fts",
    "space_members","pinned_messages","audit_logs","encryption_keys","bundles",
    "message_attachments","friend_requests","server_invites","dm_participants",
    "spaces_members","user_settings","typing_indicators","read_receipts",
    "presence","reports","bans","roles"
)
Write-Host "--- Table inventory (service_role) ---"
$existing = @()
$missing = @()
foreach ($t in $tables) {
    try {
        $r = Invoke-RestMethod -Uri "$url/rest/v1/$($t)?id=gt.00000000-0000-0000-0000-000000000000&limit=1" -Method GET -Headers $headers -TimeoutSec 8
        Write-Host "  EXISTS: $t"
        $existing += $t
    } catch {
        $code = $_.Exception.Response.StatusCode.value__
        if ($code -eq 404) {
            Write-Host "  MISSING: $t"
            $missing += $t
        } else {
            Write-Host "  HTTP $code : $t"
        }
    }
}
Write-Host ""
Write-Host "Summary: existing=$($existing.Count) missing=$($missing.Count)"
Write-Host "Missing tables:"
foreach ($m in $missing) { Write-Host "  - $m" }
