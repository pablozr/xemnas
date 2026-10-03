# Evidências da avaliação operacional

Os cinco `critic-*.md` são relatos independentes Luna medium, anteriores a alguns retestes do coordenador. O [relatório auditado](../avaliacao-repositorio-real.md) tem precedência para o estado final e corrige terminologia, limites e divergências.

- `logs/`: exportações GUI, respostas API/MCP, logs de operações e sessão real. Datas/IDs pertencem ao experimento local; não contêm token de API nem credencial.
- `evidencias/`: screenshots somente do Xemnas, selecionados nas duas rodadas; não incluem outras janelas pessoais.
- `core/`: auditoria estática, controle ADR independente e fixture arquitetural sintética e [retomada do release](core/retomada-release.md), não orientação upstream.
- `inventario-capacidades.md`: vinte capacidades implementadas e pontos de entrada; não é certificado de cobertura.

Não foram versionados banco SQLite, discovery, credenciais, clone do ripgrep ou runtime. Os logs são evidência observacional; não há runner completo de reprodução empacotado.

- [Núcleo instrumentado](core/nucleo-instrumentado.md): operações reais em snapshots, fixtures explicitamente artificiais e limites de runtime.
- `logs/relation-suggestions-validation.json`: teste específico compilado, mas não executado por App Control.

- [Relações pela GUI](core/relacoes-gui-final.md): teste final com provider real e fixtures locais, supersession confirmada e proposta rejeitada; resolve a pendência de runtime dos relatos anteriores.
