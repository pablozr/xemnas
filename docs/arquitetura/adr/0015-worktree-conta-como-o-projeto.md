# ADR-0015 — Worktree do git conta como o projeto registrado

**Status:** Vigente. Aceito em 06/10/2026.

## Contexto

Agentes trabalham em worktrees (`git worktree add`). Um worktree tem outro caminho, então
captura e contexto falhavam como "projeto não registrado", e as edições feitas ali se perdiam
ou perdiam o caminho que casa com os padrões dos componentes.

## Decisão

1. **A identidade de um diretório é o `git rev-parse --git-common-dir`.** Dois diretórios com
   o mesmo diretório comum são o mesmo repositório. A API local resolve o caminho do projeto
   por essa identidade em capturas, contexto e `/v1/agent/*`.
2. **Outros repositórios continuam recusados.** Só o repositório do projeto e seus worktrees
   valem. Sem git ou fora de um repositório, não há identidade e vale o caminho registrado.
3. **O git roda sem shell, com limite de 2 s e cache** (256 diretórios, 5 minutos, erros
   incluídos), para não pesar nem travar a API.
4. **O hook `stop` do Claude Code guarda edições feitas em outro worktree da sessão**, com o
   caminho relativo à raiz desse worktree, para o diff mapear aos mesmos componentes.

## Consequências

- Código em `crates/application/src/repo_identity.rs` e `apps/mcp-server/src/hook/worktree.rs`.
- Os recibos de captura continuam indexados pelo local do próprio projeto.
- Um repositório movido ou clonado de novo tem outra identidade e não é reconhecido sozinho.

## Alternativas rejeitadas

- **Registrar cada worktree como projeto:** fragmentaria decisões e regras do mesmo código.
- **Casar pelo nome da pasta:** daria falso positivo entre repositórios diferentes.
