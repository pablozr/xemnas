# 32 — Reenvio das capturas rejeitadas por pasta não cadastrada (backfill)

**What to build:** Recuperar as capturas que versões antigas descartaram em `outbox/rejected/` com `project_not_registered` (só o diagnóstico ficou; o payload não), relendo as sessões do Claude Code.

**Blocked by:** decisão do usuário. Só vale a pena se ele quiser as capturas antigas; o backup da outbox existe em `%LOCALAPPDATA%\xemnas\outbox-backup-2026-11-08`. Não executar sem o ok.

**Status: open**

- [ ] Comando `xemnas-mcp hook backfill --rejected-dir <dir> [--dry-run]` (`--dry-run` por padrão) que relê `~/.claude/projects/**/*.jsonl` pelo mesmo caminho do Stop, calcula `sha256(idempotency_key)` de cada turno e grava em `pending/` só os que casam com um nome em `rejected/`.
- [ ] Os envelopes de pastas ainda não cadastradas caem na quarentena; os demais entram pelo drain normal. O dedup do ingest torna o reenvio inofensivo.
- [ ] Relatório de contagem: casados, não encontrados, já aceitos.

**Aceite:** teste com transcript sintético e `rejected/` sintético (casa só o que deve); `--dry-run` não escreve nada; rodar duas vezes não duplica nada.
