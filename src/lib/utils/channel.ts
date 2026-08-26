/**
 * Pure utility: resolve a channel ID to its display name.
 * No store coupling — testable and reusable.
 */
export function channelNameFor(
  channelsBySpace: Record<string, Array<{ id: string; name: string; channelType: string }>>,
  dmChannels: Array<{ id: string; name: string }>,
  activeSpaceId: string | null,
  channelId: string | null
): string {
  if (!channelId) return 'Ses Kanalı';

  if (activeSpaceId) {
    const channels = channelsBySpace[activeSpaceId] ?? [];
    const found = channels.find(c => c.id === channelId);
    if (found) return found.name;
  }

  // Fallback: search in all spaces
  if (channelsBySpace) {
    for (const sId of Object.keys(channelsBySpace)) {
      const found = channelsBySpace[sId]?.find(c => c.id === channelId);
      if (found) return found.name;
    }
  }

  const dm = dmChannels?.find(c => c.id === channelId);
  if (dm) return dm.name;

  return 'Ses Kanalı';
}
