-- ============================================================================
-- veilanon — exec_sql RPC fonksiyonu (sadece service_role kullanabilir)
-- Bu fonksiyon SQL Editor'dan dinamik SQL çalıştırmak için gereklidir
-- ============================================================================

-- exec_sql fonksiyonu oluştur
CREATE OR REPLACE FUNCTION public.exec_sql(sql text)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
  EXECUTE sql;
END;
$$;

-- Sadece service_role kullanıcıları kullanabilir
REVOKE ALL ON FUNCTION public.exec_sql(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.exec_sql(text) TO service_role;

-- Fonksiyonun doğru çalıştığını doğrula
SELECT public.exec_sql('SELECT 1');
