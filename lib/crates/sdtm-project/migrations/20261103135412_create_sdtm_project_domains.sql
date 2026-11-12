-- sdtm_project_domains: one row per (version, domain code).
--
-- The domain is a complete copy: it carries its own `category` and
-- `descriptions` rather than pointing at a standard domain and
-- patching it. `key_variables` is a JSONB array of variable codes
-- kept in the order the caller supplied; every code it names must be
-- a variable of this same domain (enforced by the usecase, not by
-- the database).

CREATE TABLE IF NOT EXISTS sdtm_project_domains (
    id                       BIGSERIAL PRIMARY KEY,
    sdtm_project_version_id  BIGINT      NOT NULL REFERENCES sdtm_project_versions(id) ON DELETE CASCADE,
    domain_code              TEXT        NOT NULL,
    category                 TEXT        NOT NULL,
    descriptions             JSONB       NOT NULL DEFAULT '[]'::jsonb,
    key_variables            JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (sdtm_project_version_id, domain_code),
    CONSTRAINT sdtm_project_domains_category_check CHECK (
        category IN (
            'Special Purpose',
            'Interventions',
            'Events',
            'Findings',
            'Trial Design',
            'Relationships',
            'Study Reference'
        )
    ),
    CONSTRAINT sdtm_project_domains_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array'),
    CONSTRAINT sdtm_project_domains_key_variables_is_array CHECK (jsonb_typeof(key_variables) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_domains_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_domains_updated_at
BEFORE UPDATE ON sdtm_project_domains
FOR EACH ROW EXECUTE FUNCTION sdtm_project_domains_set_updated_at();
