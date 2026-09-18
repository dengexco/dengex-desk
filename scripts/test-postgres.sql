\set ON_ERROR_STOP on
-- Disposable fixtures in one rolled-back transaction. Run only in lab DB.
BEGIN;
CREATE ROLE dx_rls_test NOLOGIN NOBYPASSRLS;
GRANT USAGE ON SCHEMA public TO dx_rls_test;
GRANT SELECT,INSERT ON tenants,branches,device_groups TO dx_rls_test;
INSERT INTO tenants(id,customer_code,name,status) VALUES
 ('00000000-0000-0000-0000-000000000001','rls-test-a','Test A','active'),
 ('00000000-0000-0000-0000-000000000002','rls-test-b','Test B','active');
INSERT INTO branches(tenant_id,id,name) VALUES
 ('00000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000001','A'),
 ('00000000-0000-0000-0000-000000000002','10000000-0000-0000-0000-000000000002','B');
SET LOCAL ROLE dx_rls_test;
DO $$ BEGIN
 IF (SELECT count(*) FROM tenants)<>0 THEN RAISE EXCEPTION 'missing context exposed tenants'; END IF;
END $$;
SELECT set_config('app.tenant_id','00000000-0000-0000-0000-000000000001',true);
DO $$ BEGIN
 IF (SELECT count(*) FROM tenants)<>1 THEN RAISE EXCEPTION 'tenant filter failed'; END IF;
 IF (SELECT count(*) FROM branches)<>1 THEN RAISE EXCEPTION 'branch filter failed'; END IF;
 BEGIN
   INSERT INTO branches(tenant_id,id,name) VALUES('00000000-0000-0000-0000-000000000002','10000000-0000-0000-0000-000000000003','intrusion');
   RAISE EXCEPTION 'cross tenant write succeeded';
 EXCEPTION WHEN insufficient_privilege THEN NULL;
 END;
 BEGIN
   INSERT INTO device_groups(tenant_id,id,branch_id,name) VALUES('00000000-0000-0000-0000-000000000001','20000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000002','cross-fk');
   RAISE EXCEPTION 'cross tenant foreign key succeeded';
 EXCEPTION WHEN foreign_key_violation THEN NULL;
 END;
END $$;
RESET ROLE;
INSERT INTO audit_events(tenant_id,id,event_type,result,reason_code) VALUES('00000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000001','test','denied','test_fixture');
DO $$ BEGIN
 BEGIN
  UPDATE audit_events SET reason_code='altered';
  RAISE EXCEPTION 'audit mutation succeeded';
 EXCEPTION WHEN raise_exception THEN
  IF SQLERRM<>'audit_events is append only' THEN RAISE; END IF;
 END;
END $$;
ROLLBACK;
-- SET LOCAL must not survive transaction completion on this same connection.
DO $$ BEGIN
 IF NULLIF(current_setting('app.tenant_id',true),'') IS NOT NULL THEN RAISE EXCEPTION 'tenant context leaked after transaction'; END IF;
END $$;
SELECT 'PASS: tenant reads/writes, composite FK, append-only audit, transaction context cleanup' AS result;
