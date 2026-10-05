# Memória descritiva e router inicial

Data: 04/10/2026.
Status: implementação integrada, revisão independente concluída; validação com
limitações SAC explicitadas abaixo. Contrato: [ADR-0011](../arquitetura/adr/0011-observacoes-descritivas-de-manifests.md).

## O que funciona

- Cargo.toml/package.json e membros declarados geram observações sem confirmação,
  sem criar regras/decisões e sem chamar provider. Escopo limitado a esses manifests.
- Fontes e requisitos declarados têm hash/campo/versão/política; herança Cargo
  conserva suportes da raiz. Não se infere versão instalada ou obrigação.
- Cache persistente evita parsing de bytes/política inalterados após reopen.
- Registro/captura agenda refresh; startup e reconciler de 60 s percorrem projetos
  com cursor justo, até 64 agendamentos por rodada. A listagem ainda lê todos os
  projetos; esse limite não é paginação SQL.
- Worker local separado evita aguardar a fila de chamadas IA. Purge durante refresh
  cancela o job removido sem encerrar o worker dos demais projetos.
- Seleção por tarefa com nomes explícitos e por arquivos funciona sem vínculo
  normativo no mapa. Normas não são expulsas para acomodar descrições.
- Dirty/falha/quotas levam a abstenção descritiva, sem apagar a história como se
  a fonte tivesse sido comprovadamente removida. Correções chegam à sessão que
  recebeu a versão anterior; revalidação não é chamada de invalidação.

## Testes e review

Lote anterior fechado: material usa tokens, detector de arquitetura distingue
conversão `to_rgb8` de construtor. Frontend 59/59 e arquitetura 13/13 passaram.
Wiring novo: check, Clippy, fmt e 79 testes desktop/arquitetura passaram naquela
validação. Check desktop e fmt foram repetidos após as correções backend.

Estado final storage: 136 testes passaram; executáveis `extraction` e `jobs`
permaneceram bloqueados por SAC/4551 após tentativa de metadata isolada. Não há
uma suíte integralmente verde. Observações: 9 testes passaram; router: 6 passaram,
benchmark executado separadamente. Corpus descritivo cobre 8/16 oráculos de seleção
mais regressões de lifecycle; não certifica toda a matriz.

Review independente reproduziu e corrigiu escape por junction, fonte incerta
tratada como remoção, membership ressuscitado, herança/cache, suporte da raiz,
purge do worker e semântica/CAS de correções. Review final delimitado não deixou
P1/P2 pendentes. Teste Windows cobre raiz substituída por junction; não representa
prova completa contra corridas adversariais. Nenhum dado pessoal/provider real.

O índice Git e dois arquivos zerados foram recuperados com autorização explícita,
preservando backups; demais WIPs e arquivos do usuário foram mantidos.

## Medição atual

Somente `build_pack`, debug otimizado; não inclui refresh, parser, transporte,
tempo de agente ou esforço humano. 30 warmups e 300 amostras por benchmark:

| Corpus | p50 | p95 | p99 |
| --- | --- | --- | --- |
| Descritivo | 185 µs | 327 µs | 468 µs |
| Normativo original | 420 µs | 725 µs | — |

Corpus normativo inalterado: precisão 17/26 (65,38%), recall 17/18 (94,44%),
negativos contaminados 2/15. Qualidade dessa busca não melhorou neste slice;
variação de latência entre execuções não demonstra ganho ou regressão por si só.
Rota descritiva não recebe provider; custo adicional de tokens faturados/tempo
humano ainda não medido. Não anunciar produtividade comprovada.

## Próximo ciclo

Completar oráculos reservados e medições de refresh/prepare/HTTP, executar targets
bloqueados, comparar tarefas humanas e então experimentar julgamento de relevância
por IA somente nas ambiguidades da shortlist. Não ampliar para .env, configuração
arbitrária ou geração normativa automática.
