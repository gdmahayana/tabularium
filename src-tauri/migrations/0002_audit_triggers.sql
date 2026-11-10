-- 0002_audit_triggers.sql — Catatan audit bersifat tambah-saja (append-only)
--
-- Audit tidak boleh dimatikan atau diedit (dokumen induk 6.4). Selain lewat
-- layanan audit di Rust, penjagaan dipasang di tingkat database: catatan audit
-- tidak dapat diubah maupun dihapus oleh siapa pun yang membuka berkas ini.

CREATE TRIGGER audit_log_tolak_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'Catatan audit tidak dapat diubah.');
END;

CREATE TRIGGER audit_log_tolak_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'Catatan audit tidak dapat dihapus.');
END;