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

Implementar somente o ticket atual e o escopo do MVP aprovado; funcionalidades futuras exigem decisão explícita.

### Violation

Introduzir cloud, grafo, RAG, MCP, sincronização, multiusuário, daemon, novos adapters ou mutação automática do Project fora do escopo aprovado.

### Allowed

Suporte técnico estritamente necessário para o ticket, testes, documentação e correções de segurança.