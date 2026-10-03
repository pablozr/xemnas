# Decisões de engenharia do RelayDesk

## ADR-001: persistência compartilhada (substituída por ADR-002)
Escolha: Usar PostgreSQL com servidor separado para a fila local.
Motivo: A equipe inicialmente planejava compartilhar a fila entre máquinas.

## ADR-002: persistência individual (vigente; substitui ADR-001)
Escolha: Usar SQLite embutido para a persistência local, sem serviço de banco separado.
Motivo: O escopo mudou para desktop individual que precisa operar offline em campo; não depender de rede nem instalar um daemon. PostgreSQL foi abandonado junto com a premissa de fila compartilhada.

## ADR-003: busca (vigente)
Escolha: Usar FTS5 do SQLite para busca lexical dos registros nesta primeira versão.
Motivo: Evitar baixar modelos e consumir memória num laptop de 8 GB. A busca lexical não garante encontrar sinônimos; busca semântica ainda não foi aprovada.
