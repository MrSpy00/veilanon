-- ============================================================================
-- veilanon test verisi temizleme (Clean Test Data)
-- Migration: 20260818160000_truncate_test_data.sql
-- ============================================================================
-- Bu migration şemayı, tabloları ve RLS politikalarını KORUR;
-- yalnızca şu ana kadar oluşan test kayıtlarını temizler.

truncate table public.messages cascade;
truncate table public.channel_members cascade;
truncate table public.friendships cascade;
truncate table public.files cascade;
truncate table public.role_members cascade;
truncate table public.roles cascade;
truncate table public.invites cascade;
truncate table public.channels cascade;
truncate table public.memberships cascade;
truncate table public.spaces cascade;
truncate table public.devices cascade;
