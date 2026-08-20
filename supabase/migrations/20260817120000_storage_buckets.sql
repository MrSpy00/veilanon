-- ============================================================================
-- veilanon storage schema — bucket & RLS policies for opaque encrypted blobs
-- ============================================================================

-- Ensure the 'files' bucket exists and is public
insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values ('files', 'files', true, 52428800, null)
on conflict (id) do update set public = true;

-- Drop existing policies if any
drop policy if exists "Allow public read on files bucket" on storage.objects;
drop policy if exists "Allow public insert on files bucket" on storage.objects;
drop policy if exists "Allow public update on files bucket" on storage.objects;
drop policy if exists "Allow public delete on files bucket" on storage.objects;

-- Allow all users (anon & authenticated) to download encrypted blobs from 'files'
create policy "Allow public read on files bucket"
on storage.objects for select
to anon, authenticated
using (bucket_id = 'files');

-- Allow all users to upload encrypted blobs to 'files'
create policy "Allow public insert on files bucket"
on storage.objects for insert
to anon, authenticated
with check (bucket_id = 'files');

-- Allow all users to update their files in 'files'
create policy "Allow public update on files bucket"
on storage.objects for update
to anon, authenticated
using (bucket_id = 'files');

-- Allow all users to delete files from 'files'
create policy "Allow public delete on files bucket"
on storage.objects for delete
to anon, authenticated
using (bucket_id = 'files');
