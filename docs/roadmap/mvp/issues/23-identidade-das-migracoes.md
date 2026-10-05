# 23 — Identidade das migrações

**What to build:** O banco registra o nome de cada migração aplicada, não só o
número. Ao abrir, o app detecta quando o banco veio de builds com outra
numeração e para com uma mensagem clara, sem aplicar nada pela metade.

**Blocked by:** nada. Trabalho da sessão de backend (`crates/storage-sqlite`).

**Status: open**

## Motivo

Em 05/10/2026 o banco do usuário tinha passado por builds de branches com
numerações diferentes (antes e depois do merge que renumerou as migrações 21 a
39). Estava na versão 35 com tabelas que, na numeração atual, são da 36 a 38, e
sem três migrações que nunca rodaram (`jobs.run_after`, `job_settings`,
`qualifiers`). O app novo falhou em `CREATE TABLE review_targets` e a tela pediu
para verificar o espaço em disco. O reparo manual foi aplicar só o que faltava e
marcar o resto, conferido contra um banco novo.

## Escopo

- [ ] `schema_migrations` ganha o nome do arquivo (e, se barato, um hash do
  SQL). Bancos antigos sem nome são preenchidos uma vez pela numeração atual,
  só quando o esquema confere com ela.
- [ ] Ao abrir, número e nome divergentes viram um erro tipado
  (`StorageError::MigrationMismatch`, com a versão e os dois nomes), antes de
  qualquer migração rodar. A tela de erro de inicialização
  (`fix/startup-error-screen`) mostra esse caso.
- [ ] Teste: um banco migrado com outra numeração é recusado sem alteração.
- [ ] Regra registrada em `docs/arquitetura/`: migração nova sempre no fim,
  nunca renumerar as que já existem em `master`; um merge que precisar
  renumerar acrescenta uma migração de reparo, como a 39.

**Aceite:** abrir um banco de outra numeração nunca aplica DDL e diz qual
versão diverge; o teste cobre o caso real de 05/10/2026.
