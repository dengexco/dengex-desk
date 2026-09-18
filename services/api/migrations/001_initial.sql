-- Schema foundation; not wired to the phase-0 health API.
-- Run as migration owner. App connections must SET LOCAL app.tenant_id inside
-- a transaction after server-side authorization, and use a non-owner role.
BEGIN;
CREATE TABLE tenants (
 id uuid PRIMARY KEY, customer_code text UNIQUE NOT NULL, name text NOT NULL,
 status text NOT NULL CHECK(status IN ('active','suspended','revoked')),
 timezone text NOT NULL DEFAULT 'Europe/Istanbul', created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE users (
 id uuid PRIMARY KEY, issuer text NOT NULL, subject text NOT NULL,
 display_name text NOT NULL, UNIQUE(issuer,subject)
);
CREATE TABLE branches (
 tenant_id uuid NOT NULL REFERENCES tenants(id), id uuid NOT NULL,
 name text NOT NULL, PRIMARY KEY(tenant_id,id)
);
CREATE TABLE device_groups (
 tenant_id uuid NOT NULL REFERENCES tenants(id), id uuid NOT NULL, branch_id uuid,
 name text NOT NULL, PRIMARY KEY(tenant_id,id),
 FOREIGN KEY(tenant_id,branch_id) REFERENCES branches(tenant_id,id)
);
CREATE TABLE memberships (
 tenant_id uuid NOT NULL REFERENCES tenants(id), user_id uuid NOT NULL REFERENCES users(id),
 role text NOT NULL CHECK(role IN ('support_manager','technician','customer_admin','customer_user','auditor')),
 revoked_at timestamptz, PRIMARY KEY(tenant_id,user_id)
);
CREATE TABLE devices (
 tenant_id uuid NOT NULL REFERENCES tenants(id), id uuid NOT NULL,
 support_id text UNIQUE NOT NULL, name text NOT NULL, group_id uuid,
 os text NOT NULL CHECK(os IN ('windows','macos')), architecture text NOT NULL,
 os_version text NOT NULL, agent_version text NOT NULL,
 capabilities jsonb NOT NULL DEFAULT '{}'::jsonb,
 policy_version bigint NOT NULL DEFAULT 1 CHECK(policy_version>0),
 acknowledged_policy_version bigint NOT NULL DEFAULT 0,
 unattended_enabled boolean NOT NULL DEFAULT false,
 password_version bigint NOT NULL DEFAULT 0, password_changed_at timestamptz,
 enrollment_approved_at timestamptz NOT NULL,
 revoked_at timestamptz, last_seen_at timestamptz,
 PRIMARY KEY(tenant_id,id), FOREIGN KEY(tenant_id,group_id) REFERENCES device_groups(tenant_id,id)
);
CREATE TABLE device_keys (
 tenant_id uuid NOT NULL, device_id uuid NOT NULL, id uuid NOT NULL,
 public_key bytea NOT NULL CHECK(octet_length(public_key)=32),
 fingerprint bytea UNIQUE NOT NULL CHECK(octet_length(fingerprint)=32),
 created_at timestamptz NOT NULL DEFAULT now(), revoked_at timestamptz,
 PRIMARY KEY(tenant_id,id), FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id)
);
CREATE TABLE technician_assignments (
 tenant_id uuid NOT NULL, user_id uuid NOT NULL, device_id uuid NOT NULL,
 capabilities text[] NOT NULL, revoked_at timestamptz,
 PRIMARY KEY(tenant_id,user_id,device_id),
 FOREIGN KEY(tenant_id,user_id) REFERENCES memberships(tenant_id,user_id),
 FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id)
);
CREATE TABLE enrollment_tokens (
 tenant_id uuid NOT NULL REFERENCES tenants(id), id uuid NOT NULL,
 token_hash bytea UNIQUE NOT NULL CHECK(octet_length(token_hash)=32), group_id uuid,
 expires_at timestamptz NOT NULL, consumed_at timestamptz, revoked_at timestamptz,
 created_by uuid NOT NULL, PRIMARY KEY(tenant_id,id),
 FOREIGN KEY(tenant_id,created_by) REFERENCES memberships(tenant_id,user_id),
 FOREIGN KEY(tenant_id,group_id) REFERENCES device_groups(tenant_id,id)
);
CREATE TABLE access_policies (
 tenant_id uuid NOT NULL REFERENCES tenants(id), id uuid NOT NULL, version bigint NOT NULL CHECK(version>0),
 capabilities text[] NOT NULL, access_schedule jsonb NOT NULL DEFAULT '{}'::jsonb,
 audit_retention_days integer NOT NULL DEFAULT 90 CHECK(audit_retention_days BETWEEN 1 AND 3650),
 PRIMARY KEY(tenant_id,id,version)
);
CREATE TABLE device_policy_versions (
 tenant_id uuid NOT NULL, device_id uuid NOT NULL, version bigint NOT NULL,
 policy_id uuid NOT NULL, policy_version bigint NOT NULL,
 requested_at timestamptz NOT NULL DEFAULT now(), acknowledged_at timestamptz,
 PRIMARY KEY(tenant_id,device_id,version),
 FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id),
 FOREIGN KEY(tenant_id,policy_id,policy_version) REFERENCES access_policies(tenant_id,id,version)
);
CREATE TABLE sessions (
 tenant_id uuid NOT NULL, id uuid NOT NULL, device_id uuid NOT NULL, technician_id uuid NOT NULL,
 state text NOT NULL CHECK(state IN ('requested','awaiting_consent','negotiating','authenticating','active','reconnecting','ended','failed')),
 policy_version bigint NOT NULL, capabilities text[] NOT NULL,
 consent_type text CHECK(consent_type IN ('attended','unattended')),
 requested_at timestamptz NOT NULL DEFAULT now(), started_at timestamptz, ended_at timestamptz,
 state_deadline timestamptz NOT NULL, lease_expires_at timestamptz,
 PRIMARY KEY(tenant_id,id), FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id),
 FOREIGN KEY(tenant_id,technician_id) REFERENCES memberships(tenant_id,user_id)
);
CREATE UNIQUE INDEX one_controller_per_device ON sessions(tenant_id,device_id)
 WHERE state NOT IN ('ended','failed') AND 'control_input'=ANY(capabilities);
CREATE TABLE session_participants (
 tenant_id uuid NOT NULL, session_id uuid NOT NULL, user_id uuid NOT NULL,
 joined_at timestamptz NOT NULL DEFAULT now(), left_at timestamptz,
 PRIMARY KEY(tenant_id,session_id,user_id),
 FOREIGN KEY(tenant_id,session_id) REFERENCES sessions(tenant_id,id),
 FOREIGN KEY(tenant_id,user_id) REFERENCES memberships(tenant_id,user_id)
);
CREATE TABLE consent_events (
 tenant_id uuid NOT NULL, id uuid NOT NULL, session_id uuid NOT NULL,
 accepted boolean NOT NULL, capabilities text[] NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(tenant_id,id),
 FOREIGN KEY(tenant_id,session_id) REFERENCES sessions(tenant_id,id)
);
CREATE TABLE audit_events (
 tenant_id uuid NOT NULL REFERENCES tenants(id), id uuid NOT NULL,
 actor_id uuid, device_id uuid, session_id uuid,
 event_type text NOT NULL, result text NOT NULL, reason_code text NOT NULL,
 consent_type text, policy_version bigint, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(tenant_id,id),
 FOREIGN KEY(tenant_id,actor_id) REFERENCES memberships(tenant_id,user_id),
 FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id),
 FOREIGN KEY(tenant_id,session_id) REFERENCES sessions(tenant_id,id)
);
CREATE TABLE transfer_summaries (
 tenant_id uuid NOT NULL, id uuid NOT NULL, session_id uuid NOT NULL,
 direction text NOT NULL CHECK(direction IN ('send','receive')), byte_count bigint NOT NULL CHECK(byte_count>=0),
 result text NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(tenant_id,id),
 FOREIGN KEY(tenant_id,session_id) REFERENCES sessions(tenant_id,id)
);
CREATE TABLE agent_releases (
 id uuid PRIMARY KEY, version text NOT NULL, platform text NOT NULL, architecture text NOT NULL,
 channel text NOT NULL CHECK(channel IN ('stable','beta')), manifest_signature bytea NOT NULL,
 package_digest bytea NOT NULL, min_security_version bigint NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(version,platform,architecture,channel)
);
CREATE TABLE update_rollouts (
 tenant_id uuid NOT NULL REFERENCES tenants(id), id uuid NOT NULL, release_id uuid NOT NULL REFERENCES agent_releases(id),
 percentage integer NOT NULL CHECK(percentage BETWEEN 0 AND 100), maintenance_window jsonb NOT NULL,
 state text NOT NULL CHECK(state IN ('draft','active','paused','ended')), PRIMARY KEY(tenant_id,id)
);
-- Tenant context is transaction-local, never a pooled connection default.
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['branches','device_groups','memberships','devices','device_keys','technician_assignments','enrollment_tokens','access_policies','device_policy_versions','sessions','session_participants','consent_events','audit_events','transfer_summaries','update_rollouts'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY tenant_scope ON %I USING (tenant_id = NULLIF(current_setting(''app.tenant_id'',true),'''')::uuid) WITH CHECK (tenant_id = NULLIF(current_setting(''app.tenant_id'',true),'''')::uuid)',tab);
 END LOOP;
END $$;
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenants FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_scope ON tenants USING(id=NULLIF(current_setting('app.tenant_id',true),'')::uuid)
 WITH CHECK(id=NULLIF(current_setting('app.tenant_id',true),'')::uuid);
CREATE FUNCTION reject_audit_mutation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 RAISE EXCEPTION 'audit_events is append only'; END $$;
CREATE TRIGGER audit_immutable BEFORE UPDATE OR DELETE ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_audit_mutation();
-- Application role provisioning is separate; never grant table-owner or
-- BYPASSRLS privileges. Audit writer gets INSERT; API audit reader gets SELECT.
-- No password/verifier, raw token, keypress, clipboard or file-content columns.
COMMIT;
