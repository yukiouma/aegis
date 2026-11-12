-- sdtm_project_variables: one row per (domain, variable code).
--
-- The variable is a complete copy: it carries its own type, core,
-- role, controlled terminology and descriptions rather than pointing
-- at a standard variable and patching it. `conversion_rules` is a
-- JSONB array of {source, rule} objects saying where the value comes
-- from and how it is derived.

CREATE TABLE IF NOT EXISTS sdtm_project_variables (
    id                       BIGSERIAL PRIMARY KEY,
    sdtm_project_domain_id  BIGINT      NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE,
    variable_code            TEXT        NOT NULL,
    variable_sequence        BIGINT      NOT NULL,
    variable_controlled      JSONB,
    variable_type            TEXT        NOT NULL,
    variable_core            TEXT        NOT NULL,
    variable_role            TEXT,
    descriptions             JSONB       NOT NULL DEFAULT '[]'::jsonb,
    conversion_rules         JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (sdtm_project_domain_id, variable_code),
    CONSTRAINT sdtm_project_variables_type_check CHECK (variable_type IN ('Numeric', 'Character')),
    CONSTRAINT sdtm_project_variables_core_check CHECK (variable_core IN ('Req', 'Exp', 'Perm', 'Supp')),
    CONSTRAINT sdtm_project_variables_role_check CHECK (
        variable_role IS NULL OR variable_role IN (
            'Identifier',
            'Topic',
            'Timing',
            'Record Qualifier',
            'Synonym Qualifier',
            'Variable Qualifier',
            'Grouping Qualifier',
            'Rule'
        )
    ),
    CONSTRAINT sdtm_project_variables_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array'),
    CONSTRAINT sdtm_project_variables_conversion_rules_is_array CHECK (jsonb_typeof(conversion_rules) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_variables_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_variables_updated_at
BEFORE UPDATE ON sdtm_project_variables
FOR EACH ROW EXECUTE FUNCTION sdtm_project_variables_set_updated_at();
