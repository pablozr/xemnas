-- The workspace root and CI components used to be created with a Portuguese
-- sentence as their description. They carry none now: the interface writes it
-- in its language and the model reads a fixed English text (ADR-0017).
-- Forward only: it clears a description only when it is exactly the old
-- literal, on a component whose patterns say it is the root (`*`) or a CI
-- system, so nothing written by hand is touched.
UPDATE entities
SET description = ''
WHERE kind = 'component'
  AND description = 'Arquivos da raiz: manifesto do workspace, toolchain e configuração comum'
  AND EXISTS (
      SELECT 1 FROM entity_patterns
      WHERE entity_patterns.entity_id = entities.entity_id AND entity_patterns.pattern = '*'
  );

UPDATE entities
SET description = ''
WHERE kind = 'component'
  AND EXISTS (
      SELECT 1 FROM entity_patterns
      WHERE entity_patterns.entity_id = entities.entity_id
        AND entity_patterns.pattern IN (
            '.github/workflows/**', '.gitlab-ci.yml', '.circleci/**',
            'azure-pipelines.yml', 'Jenkinsfile', '.buildkite/**'
        )
        AND entities.description = 'Integração contínua e publicação (' || entity_patterns.pattern || ')'
  );
