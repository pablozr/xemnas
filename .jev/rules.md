```yaml
rules:
  ARCH-001:
    source: user
  PRIV-001:
    source: user
  RUST-001:
    source: user
  RUST-002:
    source: user
  RUST-003:
    source: user
  RUST-004:
    source: user
  ASYNC-001:
    source: user
  TEST-001:
    source: user
  SCOPE-001:
    source: user
  ARCH-002:
    source: user
  CONTRACT-001:
    source: user
  DATA-001:
    source: user
  STYLE-001:
    source: user
  STYLE-002:
    source: user
```

## ARCH-001

severity: error

### Rule

Preservar as fronteiras aprovadas: o domínio não conhece UI, SQLite, HTTP, OpenCode nem providers de IA; UI usa casos de uso internos e HTTP local é somente fronteira de adapters.

### Violation

Adicionar dependência ou acoplamento entre essas camadas, ou usar HTTP como comunicação interna da UI.

### Allowed

Implementações de infraestrutura de ports definidos pela aplicação e dependências explicitamente aprovadas pela especificação.

## PRIV-001

severity: error

### Rule

Proteger dados locais e segredos: nunca registrar, persistir em texto simples, exportar ou expor tokens, credenciais, prompts completos, conversas, diffs ou conteúdo sensível sem autorização explícita.

### Violation

Adicionar logs, erros, diagnósticos, fixtures ou armazenamento que revelem dados sensíveis; enviar dados externos sem consentimento.

### Allowed

Dados sintéticos sanitizados em testes e metadados mínimos necessários, sem conteúdo sensível.

## RUST-001

severity: error

### Rule

Falhas recuperáveis de entrada, I/O, rede, banco ou ambiente devem retornar Result com contexto; panic!, unwrap() e expect() não devem tratar essas falhas em produção.

### Violation

Introduzir panic, unwrap ou expect em caminho de produção que possa falhar por dados externos ou ambiente.

### Allowed

Testes e invariantes locais demonstráveis; expect() nesses casos deve explicar a invariante.

## RUST-002

severity: error

### Rule

Não introduzir unsafe no código próprio sem necessidade, encapsulamento seguro e justificativa documentada.

### Violation

Adicionar ou ampliar código unsafe sem comentário SAFETY:, invariantes explícitas e testes do caminho.

### Allowed

Integração FFI ou requisito nativo aprovado com superfície unsafe mínima e documentação adequada.

## RUST-003

severity: warning

### Rule

APIs públicas reais devem documentar seu comportamento e, quando aplicável, erros, panics e segurança.

### Violation

Expor API pública sem documentação suficiente para seu uso seguro.

### Allowed

Itens internos, código de binário sem consumidor externo e APIs temporárias explicitamente limitadas ao spike.

## RUST-004

severity: error

### Rule

Mudanças Rust devem manter formatação, Clippy sem warnings e testes relevantes; Cargo.lock deve permanecer versionado e a validação usa dependências bloqueadas.

### Violation

Introduzir código que falhe em formatação, Clippy, testes relevantes ou altere dependências sem lockfile correspondente.

### Allowed

Falhas preexistentes registradas antes da mudança, sem agravamento, e exceções locais justificadas.

## ASYNC-001

severity: error

### Rule

Operações bloqueantes ou pesadas não podem rodar na thread da UI nem bloquear futures; tasks devem ter ciclo de vida e erros observáveis.

### Violation

Executar I/O, banco, CPU pesada ou chamada externa na UI/future inadequado; descartar erros ou handles de tasks sem decisão explícita.

### Allowed

Operações curtas comprovadamente não bloqueantes e trabalho explicitamente delegado a executor, fila ou spawn_blocking.

## TEST-001

severity: error

### Rule

Mudanças comportamentais devem incluir validação proporcional e preservar os invariantes do MVP: idempotência, recuperação, privacidade, confirmação humana e ausência de mutação surpresa.

### Violation

Entregar comportamento novo ou alterado sem teste/validação proporcional, ou enfraquecer esses invariantes.

### Allowed

Mudanças estritamente mecânicas sem comportamento observável, validadas pelos checks existentes.

## SCOPE-001

severity: error

### Rule

Implementar somente o ticket atual e o escopo aprovado; funcionalidades futuras exigem decisão explícita do usuário registrada em ADR (`docs/adr/`) antes do código.

### Violation

Introduzir cloud, grafo, RAG, MCP, sincronização, multiusuário, daemon, novos adapters ou mutação automática do Project fora do escopo aprovado.

### Allowed

Suporte técnico estritamente necessário para o ticket, testes, documentação e correções de segurança.

## ARCH-002

severity: error

### Rule

Composition roots (binários em `apps/`) só montam dependências, registram handlers e fazem log; orquestração e regra de negócio vivem em casos de uso do `application`. Caminhos locais vêm só de `application::AppPaths`.

### Violation

Decidir fluxo de negócio no binário (escolher provider, checar consentimento, gravar proveniência), duplicar resolução de caminhos ou variáveis de ambiente fora de `AppPaths`, ou fazer o storage ler o filesystem fora do banco.

### Allowed

Logs, mapeamento de resultado para estado de job e leitura de configuração de runtime (por exemplo retenção) na composition root.

## CONTRACT-001

severity: error

### Rule

Os tipos de `integration-contracts` são o contrato versionado com o adapter TypeScript: suas docs geram o JSON Schema. Qualquer mudança neles regenera o schema, atualiza o adapter e mantém o teste de contrato verde.

### Violation

Editar campos ou docs desses tipos sem regenerar `adapters/opencode/schemas/`, ou mudar o contrato sem subir `schema_version`.

### Allowed

Mudanças internas que não alteram o schema gerado.

## DATA-001

severity: error

### Rule

Migrations são forward-only e transacionais. Uma migration nova atualiza junto os testes que contam migrations, a versão citada no `README.md` e em `tests/e2e/install-clean.ps1`, e testa o upgrade a partir da versão anterior com dados existentes.

### Violation

Editar migration já publicada, apagar dados em migration sem decisão explícita, ou adicionar migration sem teste de upgrade.

### Allowed

Lacunas de numeração já registradas (0007).

## STYLE-001

severity: warning

### Rule

Código legível sem comentários narrativos: nomes e funções pequenas explicam o código. Nada de comentários `//` com histórico, referência a ticket ou explicação longa. Itens públicos têm doc de uma linha objetiva (exigida por `missing_docs`).

### Violation

Adicionar blocos de comentário explicativo, referências como "ticket 12" ou docs de vários parágrafos.

### Allowed

`// SAFETY:` em `unsafe`, seção `# Errors` quando o erro não é óbvio, e as docs de `integration-contracts` (CONTRACT-001).

## STYLE-002

severity: warning

### Rule

Linhas com até 100 colunas e closures curtas. Expressão que o `rustfmt` não consegue quebrar vira função nomeada.

### Violation

Linhas longas que o `rustfmt` deixou sem formatar, como closures com lógica e strings de UI inteiras em uma linha.

### Allowed

Literais de string longos (SQL, mensagens de produto, fixtures e dados de demonstração).
