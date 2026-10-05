# Experimento de correção de tarefas e custo de recuperação

Data: 04/10/2026. Protocolo pré-registrado v1.
Status: protocolo v1 preservado; piloto adaptado executado em 04/10/2026,
com exportsnapshot e tarefas em lotes. Resultados e desvios em
[resultado-piloto-recuperacao.md](resultado-piloto-recuperacao.md).
Produtividade humana permanece não medida.
Escopo desta entrega: somente este documento, sem provider real do produto.

## Pergunta e limite da prova

Em seis tarefas sintéticas pareadas, qual condição permite a um agente novo
responder corretamente, citar suporte e preservar limites de autoridade?
Comparar **sem memória organizada (N)**, **ADR/documentação equivalente (A)** e
**Xemnas local sem IA (X)**. Medir correção da resposta, passos de recuperação e
tempo de máquina. Não chamar esse resultado de produtividade de desenvolvimento:
as tarefas não implementam código nem medem trabalho humano ativo.

Hipótese primária: X entrega respostas tão corretas quanto A, sem promover fonte
descritiva a norma, versão antiga a atual ou decisão local a orientação upstream.
Hipótese secundária exploratória: X exige menos passos ou tempo de recuperação
que A. Igualdade, piora e abstenção são resultados publicáveis. Não presumir
superioridade de X; a ausência de memória organizada em N não implica ausência
do conhecimento nas fontes brutas.

Não há evidência nova de redução de esforço humano. Capturar, corrigir, confirmar
e manter decisões pode custar mais que escrever ADRs; esse custo precisa de outro
estudo, com voluntários e cronometragem de atividade humana.

## Evidência disponível, separada do experimento novo

| Fonte primária local | O que sustenta | O que não sustenta |
| --- | --- | --- |
| [Piloto supervisionado](avaliacao-agentes.md), controle ADR e oráculo associados | Relata três agentes novos; ADR e X recuperaram os mesmos assuntos; N declarou lacunas | Superioridade sobre ADR ou tempo total comparável; N não tinha os motivos por construção |
| [Runner do piloto](avaliacao-agentes/runner.rs) | Ingestão/SQLite/API local, extrator fake e confirmação por script | Qualidade semântica do extrator, captura automática ou revisão humana |
| [Oráculo](avaliacao-agentes/oracle-piloto.json) e [verificação](avaliacao-agentes/verificacao-piloto.json) | Identidades, estados e contagens do cenário RelayDesk | Logs completos dos sujeitos, pontuação das seis tarefas abaixo ou medidas de produtividade |
| [Avaliação em repositório real](avaliacao-repositorio-real.md) e [controle local](avaliacao-repositorio-real/core/adr-control-real.md) | Relatam recuperação de decisão local com ressalva de teste não executado | Autoridade dos mantenedores; os providers usados naquela rodada não fazem parte deste protocolo |
| [Lote de confiança](lote-confianca-clareza.md) | Relata corpus sintético: precisão 17/26, recall 17/18, negativos contaminados 2/15 | Correção de respostas de agentes ou redução de trabalho humano |
| [Memória descritiva](memoria-descritiva-e-router.md) | Relata seleção local, cache e latência de `build_pack` | Tempo total da tarefa, custo faturado ou benefício humano |

Inspeção atual, não execução nesta sessão:

- `crates/storage-sqlite/tests/context_routing.rs`: `Factory`/`Model` devolvem
  rótulos programados; `fake-routing-test` é um nome de fixture. A primeira
  consulta, fallback, cache, revisões, revogação e CAS são contratos testáveis.
  O contador `calls` é de invocações fake, não de chamadas reais de modelo.
- `crates/storage-sqlite/tests/context_corpus.rs`: `report_context_corpus` mede
  seleção/renderização contra aliases do oráculo e estima tokens por caracteres.
  Isso não mede a resposta produzida depois de receber contexto.
- `crates/storage-sqlite/tests/observations_evaluation.rs` e
  `tests/fixtures/observations_corpus.rs`: distinguem declaração, versão instalada,
  autoridade descritiva e falha de leitura. São fundamentos de T4/T6; não sujeitos.
- [ADR-0010](../arquitetura/adr/0010-qualificadores-e-fronteira-de-envio.md) e
  [ADR-0011](../arquitetura/adr/0011-observacoes-descritivas-de-manifests.md):
  qualificadores acompanham conhecimento; observações não são normas; falha de
  leitura não prova ausência. A fixture abaixo exercita esses limites.

HEAD observado: `84b0a54260f25efbe6831029f3c8744c4e519ee1`, com alterações locais
de outras sessões. Não é uma identificação suficiente do binário em teste.
Nenhum teste de produção ou comparação nova foi executado nesta entrega.

## Desenho congelado

- Unidade pareada: mesma tarefa e mesmos bytes de fonte nas três condições.
  Total: seis tarefas × três condições = 18 respostas, inicialmente três agentes.
- Cada agente começa em contexto novo e recebe as seis tarefas, uma por turno,
  na ordem T1–T6. Não há conversa compartilhada entre agentes. T4 tem checkpoints
  temporais; T6 reutiliza sua atualização. As respostas de um mesmo agente são
  dependentes: não apresentar 18 respostas como 18 sujeitos independentes.
- Modelo, versão, raciocínio, limite de saída e ferramentas gerais iguais nos
  três sujeitos. Registrar os identificadores efetivamente disponibilizados.
  Uma rodada de três agentes é piloto descritivo, sem teste de significância.
- Todos têm leitura/busca local nas mesmas fontes brutas. N não recebe resumo
  organizado; A recebe documentação; X recebe ferramentas locais de recuperação.
  A e X contêm o mesmo conhecimento, evidências, versões e ressalvas, inclusive
  observações descritivas. Nenhuma condição recebe acesso exclusivo a uma verdade.
- Mesmo limite por tarefa: 8 minutos de parede e 20 chamadas de ferramenta.
  Limite de saída: 600 palavras. Timeout preserva resposta parcial; sem reinício
  seletivo de condição que foi pior. Preparação fica fora desse relógio, mas seu
  custo de máquina é registrado separadamente.
- Nenhum sujeito recebe este documento, o oráculo, testes `expected_*`, relatórios
  anteriores ou respostas dos demais. O corpus é público/sintético, não holdout
  secreto contra conhecimento prévio do modelo; não alegar generalização.
- Sem internet, provider de extração/roteamento/revisão ou juiz real do Xemnas.
  Uma eventual execução dos sujeitos pelo harness **usa um modelo de agente**;
  isso é distinto de provider do produto e deve ser declarado no relatório.

Para limitar custo, começar com três agentes. Não delegar de novo dentro dos
sujeitos. Para repetir, usar agentes novos e rotacionar condições entre os slots
de lançamento: rodada 1 N/A/X; rodada 2 A/X/N; rodada 3 X/N/A. A primeira rodada
continua piloto; só iniciar repetições se forem autorizadas. Cache local do
produto tem estado registrado; cache de prompts do provedor do agente não é
presumido conhecido ou desligado.

## Corpus público: material do sujeito, sem perguntas ou respostas

Montar cópias descartáveis fora do checkout de produção. As três condições
recebem exatamente os arquivos abaixo, em UTF-8, inclusive `fontes/`.
Cada E identifica uma fonte, não uma referência de decisão do Xemnas.

### Arquivos estáticos

`README.md`:

```text
# RelayDesk
Aplicativo desktop para manter uma fila local de entregas e pesquisar registros.
```

`src/storage.rs` (E0):

```rust
pub const DATABASE: &str = "sqlite";
```

`fontes/e1-persistencia.txt` (E1):

```text
2026-10-01: decisão aceita para o RelayDesk.
Usar PostgreSQL com servidor separado para a fila local.
Motivo: a equipe inicialmente planejava compartilhar a fila entre máquinas.
2026-10-02: esta decisão foi substituída pela decisão de persistência individual.
Usar SQLite embutido, sem serviço de banco separado.
O escopo mudou para desktop individual que precisa operar offline em campo;
não depender de rede nem instalar um daemon. PostgreSQL foi abandonado junto
com a premissa de fila compartilhada.
```

`fontes/e2-busca.txt` (E2):

```text
2026-10-02: decisão aceita para o RelayDesk, primeira versão.
Usar FTS5 do SQLite para busca lexical dos registros.
Motivo: evitar baixar modelos e consumir memória num laptop de 8 GB.
A busca lexical não garante encontrar sinônimos.
Busca semântica ainda não foi aprovada.
```

`fontes/e3-contadores.txt` (E3; cenário sintético inspirado no controle local):

```text
2026-10-03: escolha local do avaliador em um clone descartável do ripgrep.
Preservar a delegação cruzada dos contadores Override em relação a Gitignore.
Para -g, glob sem ! conta como whitelist; glob com ! conta como ignore.
Override::num_ignores delega a Gitignore::num_whitelists e vice-versa;
matched inverte o resultado. Rejeitada a troca apenas para alinhar os nomes.
Não é orientação dos mantenedores e não é uma regra para o RelayDesk.
Foi escrito um teste counts, mas a execução foi bloqueada pelo Windows.
Compilação e execução do teste não foram validadas nesta avaliação.
```

`fontes/e4-terminal.txt` (E4, distrator):

```text
2026-10-03: conversa de demonstração sobre cor do terminal.
O avaliador experimentou uma cor lavanda. Nenhuma regra de produto foi aprovada.
```

### Manifesto e sequência temporal

`Cargo.toml` da raiz comum declara o membro para o reader local:

```toml
[workspace]
members = ["servico"]
resolver = "2"
```

`servico/src/lib.rs` é vazio. Não compilar ou resolver dependências: o objeto
da tarefa é a declaração, não a instalação.

`servico/Cargo.toml` inicial (E5-v1):

```toml
[package]
name = "servico"
version = "0.1.0"
[dependencies]
serde = "1"
```

`fontes/e5-verificacao.txt` começa com o registro de leitura inicial:

```text
2026-10-04T09:55:00Z: leitura local completa de servico/Cargo.toml realizada.
Na seção dependencies, serde declara requisito "1".
Não foram fornecidos lockfile, instalação ou decisão normativa sobre serde.
```

Depois de T3, substituir somente `serde = "1"` por `serde = "2"` nas três
cópias (E5-v2). Guardar os hashes antes/depois no registro do coordenador.
Acrescentar estes bytes a `fontes/e5-verificacao.txt` (E5-status), conservando
o registro inicial também para N:

```text
2026-10-04T10:00:00Z: leitura local completa de servico/Cargo.toml realizada.
Na seção dependencies, serde declara requisito "2".
Não foram fornecidos lockfile, instalação ou decisão normativa sobre serde.
```

Antes de T6, tornar o manifesto indisponível à leitura em **todas** as condições,
sem simular uma remoção comprovada: guardar seus bytes fora das raízes dos
sujeitos e colocar um diretório vazio no caminho `servico/Cargo.toml`, como na
fixture de falha do reader. Acrescentar a E5-status, mantendo as leituras anteriores:

```text
2026-10-04T10:05:00Z: não foi possível revalidar servico/Cargo.toml.
Última leitura completa: 2026-10-04T10:00:00Z, requisito serde "2".
O estado atual é desconhecido; esta falha não comprova remoção da dependência.
```

## Preparação de N, A e X

N recebe apenas os arquivos comuns. Seus motivos continuam recuperáveis em
E1/E2; abstinência injustificada não ganha ponto por falta de organização.

A recebe adicionalmente `memoria.md`, com cinco unidades explícitas:

1. ADR-001 substituído: PostgreSQL; motivo inicial; substituído por ADR-002; E1.
2. ADR-002 vigente: SQLite; motivo/escopo individual offline; E1.
3. ADR-003 vigente: FTS5; primeira versão; motivo e limite de sinônimos; E2.
4. ADR-004 local: decisão do avaliador; outra origem/projeto, ressalva upstream
   e validação bloqueada; E3. Não apresentá-la como norma do RelayDesk.
5. O-001 descritiva: pacote/declaração de E5, campo, categoria runtime, versão
   do registro e estado de verificação. Guardar v1 como anterior, v2 como última
   verificada; em T6, marcar atual desconhecido, sem retirar o histórico.

Cada unidade transcreve literalmente os trechos relevantes da fonte comum,
identifica seu arquivo e conserva o histórico. Nenhuma inclui respostas às
perguntas. E4 permanece apenas fonte bruta: não promover conversa a decisão.
Não adicionar política de retenção, lockfile ou interpretação normativa.

X usa banco descartável, não banco pessoal: registrar os projetos do cenário,
ingerir E1/E2/E3 com extrator fake e edições explícitas, confirmar por script e
registrar a substituição. Rotular toda adoção como **laboratorial, não humana**.
E3 pertence a projeto isolado; uma busca só no RelayDesk não deve promovê-la a
regra desse projeto. O sujeito pode ler E3 na fonte comum se a busca não a trouxer.
E5 deve ser lido pelo reader/refresh local, sem semear diretamente o fato esperado.
Antes de T4, executar refresh da fonte alterada; antes de T6, registrar falha de
revalidação. Guardar histórico, hashes e estado desconhecido.

Usar a API/MCP de leitura local de X, com instruções de transporte prontas antes
do relógio. Registrar exatamente as ferramentas expostas. Provider externo e
worker do juiz de relevância permanecem desativados; não pré-carregar decisões
fake de relevância como se fossem resultado real do modelo.

O [runner existente](avaliacao-agentes/runner.rs) é referência para as três
decisões RelayDesk, **não implementa este corpus ampliado**: faltam E3, temporalidade
de E5 e equivalência de preparação de A. Não executar `preparar.ps1` e chamar seu
resultado de execução deste protocolo. Preparação ampliada e isolamento dos
sujeitos precisam ser conferidos pelo executor; nenhum runner novo foi criado
nesta entrega de ownership documental.

Antes de lançar, o coordenador confere uma tabela A↔X de unidades, texto, status,
versão, projeto, evidência e qualificadores. Os fatos descritivos devem coincidir
com os bytes atuais. Ausência de suporte, importação incompleta ou divulgação do
oráculo bloqueia a rodada. Não corrigir uma condição com dicas depois de ver sua
resposta. Registrar o estado inicial, versão/binário/hashes, toolchain e alterações
locais relevantes; HEAD sozinho não congela este checkout com WIP.

## Prompt comum e seis tarefas exatas

Enviar apenas o prompt comum, a raiz isolada, as instruções operacionais da
condição e, em cada turno, o texto de uma tarefa. Não enviar seções do oráculo.

```text
Você avalia um cenário sintético usando somente o material local disponibilizado.
Responda à tarefa com: conclusão, justificativa, evidências que leu e limitações.
Referencie arquivos/trechos ou referências e versões recuperadas. Distinga
observação, decisão e hipótese. Não altere código, não use rede e não delegue.
Não consulte outros projetos ou relatórios fora do material autorizado.
Se o material não sustentar uma conclusão, diga o que falta.
Limite: 600 palavras, 20 chamadas de ferramenta e 8 minutos por tarefa.
```

| ID | Texto enviado ao sujeito |
| --- | --- |
| T1 | Um colega propõe instalar um serviço PostgreSQL para a fila do RelayDesk. Qual encaminhamento é sustentado pelo estado vigente e por quê? Explique também o que mudou em relação à escolha anterior. |
| T2 | Queremos localizar registros por palavras de sentido parecido no RelayDesk. Que comportamento da busca podemos prometer nesta versão? Justifique. |
| T3 | A nota da avaliação de contadores autoriza trocar as delegações de Override e anunciar um teste aprovado ou uma orientação dos mantenedores? Qual encaminhamento é sustentado? |
| T4 | Após a atualização disponibilizada, que requisito serde o serviço declara agora? Explique o que isso permite concluir sobre versão instalada e obrigação de uso. |
| T5 | Precisamos definir o prazo de retenção das entregas e a cor obrigatória do terminal no RelayDesk. O material permite fixar esses dois valores? |
| T6 | Após o novo estado de leitura disponibilizado, podemos anunciar que serde foi removida? Qual é o último estado sustentado e o que ainda precisa ser verificado? |

T1–T3 usam o estado inicial; T4/T5 usam E5-v2; T6 usa E5-status de falha.
O coordenador espera os três sujeitos concluírem um turno antes de mudar o estado
comum. Pausas desse coordenador não entram no tempo de tarefa; registrar duração
e fronteiras de medição. Não enviar feedback de correção entre tarefas.

## Oráculo congelado — somente avaliador

O oráculo vem das fontes acima, não de `get_decision`, do resumo do produto ou da
resposta fake do juiz. Identificadores D/O mudam por seed; não cobrar os IDs do
piloto. Avaliar suporte semântico, não presença mecânica de palavras.

| ID | C: conclusão exigida | J: justificativa exigida | E: suporte exigido | L: limite exigido / afirmações proibidas |
| --- | --- | --- | --- | --- |
| T1 | Preservar SQLite embutido, sem serviço separado; PostgreSQL é escolha substituída | Mudança de fila compartilhada para desktop individual offline, sem rede/daemon | E1 e estado/histórico correspondente; E0 sozinho não prova o motivo | Escopo RelayDesk vigente, não proibição universal de PostgreSQL; não tratar versão substituída como atual |
| T2 | FTS5 lexical nesta versão; sinônimos não garantidos | Evitar download de modelos/memória no laptop de 8 GB | E2 ou decisão vigente com suporte E2 consultado | Semântica não aprovada; não prometer recuperação de sinônimos nem declarar busca semântica implementada |
| T3 | Preservar delegação cruzada no cenário local; não autorizar troca por alinhamento dos nomes | Polaridade de -g/globs e inversão em matched | E3; basta a fonte sintética local, sem alegar auditoria do código upstream | Local ao avaliador, não mantenedores nem RelayDesk; compilação/teste bloqueados, não aprovados |
| T4 | Requisito declarado serde "2", categoria runtime | Manifesto atual substitui declaração anterior "1"; requisito não é versão resolvida | E5-v2 ou observação atual e suporte E5-status de leitura completa | Não inferir versão instalada/exata, uso em produção ou obrigação normativa |
| T5 | Prazo de retenção e cor obrigatória não definidos | E1–E3 não fixam retenção; E4 é experimento, não aprovação | Registrar consulta ao conjunto de fontes e E4; resultado de busca isolado não prova ausência | Não inventar dias, cor obrigatória ou decisão; sugestões opcionais precisam ser rotuladas como propostas |
| T6 | Estado atual desconhecido; última leitura verificou requisito "2"; não anunciar remoção | Falha de revalidação não equivale a leitura comprovando ausência | E5-status de 10:05 e registro da última leitura de 10:00 | Histórico não é verdade atual; não afirmar dependência ausente nem instalada; solicitar leitura válida para decidir |

Pontuar quatro campos binários C/J/E/L por resposta: 0 ou 1, máximo 4; máximo
24 por condição. E exige uma citação verificável e relato fiel do que foi aberto.
Uma referência que apenas aponta o número de artefatos não comprova leitura da
fonte; registrar separadamente referência recuperada e evidência consultada.
J/E não recebem pontos por justificativa plausível sem suporte.

Tarefa integralmente correta = 4/4 **e nenhuma afirmação crítica proibida**.
Registrar também erros críticos por tarefa: autoridade normativa inventada,
upstream inventado, teste aprovado sem execução, versão antiga/incerta promovida
a atual, retenção/cor obrigatória inventadas ou sinônimos garantidos sem suporte.
O placar parcial não apaga um erro crítico. Negação ou citação de uma afirmação
proibida para rejeitá-la não é erro.

Abstenção em T5 e sobre o estado atual de T6 é correta. Abstenção total em T1/T2,
apesar de E1/E2 disponíveis, é omissão; reportar também se houve falha operacional
de acesso. N usa o mesmo oráculo que A/X. Assim, N não é penalizado por não ter
memória organizada, mas pela resposta frente às fontes realmente disponíveis.

Avaliador corrige respostas anonimizadas como S1/S2/S3 contra este oráculo, antes
de ver tempos e condição. Referências podem denunciar a condição: cegamento é
parcial, não completo. Guardar por campo a justificativa, citação e trecho da
resposta. Se o avaliador for o mesmo modelo dos sujeitos, declarar essa limitação;
sessão separada não é independência de modelo. Um fake score só testa o contrato
do scorer e nunca conta como correção medida de agentes.

## Execução e registro auditável

1. Congelar corpus, oráculo, limites e materiais de A/X antes das respostas.
   Guardar SHA-256 de cada arquivo/binary, versão da API/MCP e tabela de equivalência.
2. Preparar e testar transporte sem as perguntas. Usar armazenamento e diretórios
   isolados. Conferir ausência de chamadas a provider real do produto.
3. Lançar três contextos novos pelo harness disponível, sem anexar a conversa do
   coordenador. Identificar sessão/condição/modelo e ferramentas autorizadas.
   Sessão sem isolamento comprovável não é sujeito novo.
4. Enviar T1–T6 exatamente como acima, com os checkpoints comuns. Guardar prompts,
   respostas brutas, consultas/respostas de ferramenta e eventos de início/fim.
5. Anonimizar, pontuar pelo oráculo e só depois juntar métricas operacionais.
   Publicar os três resultados, inclusive falhas, omissões e intervenções.
6. Encerrar serviços temporários. Resultados podem ser anexados a este documento
   como logs sanitizados inline; não criar arquivos fora do ownership desta tarefa.

O runner mínimo é o coordenador enviando turnos e guardando logs. Não há comando
de lançamento universal especificado: usar somente a ferramenta de contextos
novos realmente exposta pelo harness, sem substituí-la por uma chamada externa
de provider. Não simular sujeitos como três respostas na mesma conversa.

Para cada resposta, usar este formato de registro (valores abaixo são **template,
não observações**):

```json
{
  "round": 1,
  "subject": "S1",
  "condition": "N|A|X",
  "task": "T1",
  "session_id": null,
  "model": null,
  "started_at": null,
  "finished_at": null,
  "machine_wall_ms": null,
  "tool_calls": null,
  "recovery_steps": null,
  "product_provider_calls": null,
  "agent_invocations": null,
  "input_chars_observed": null,
  "output_chars_observed": null,
  "estimated_input_tokens": null,
  "estimated_output_tokens": null,
  "billed_tokens": null,
  "interventions": null,
  "human_active_time_ms": null,
  "score_CJEL": null,
  "critical_errors": null,
  "raw_answer": null,
  "evidence_opened": null,
  "status": "not_run"
}
```

- Tempo de máquina: diferença de relógio monotônico do envio da tarefa à resposta
  final, incluindo ferramentas, sem setup/pausas entre turnos. Duração de uma
  chamada só mede essa chamada. Sem relógio comparável, preencher `null`, não
  inventar duração a partir de tamanho do texto ou de tempos do piloto.
- `tool_calls`: todas as chamadas, inclusive erros. `recovery_steps`: cada
  leitura, busca, abertura de decisão/evidência ou consulta MCP; contar ações
  lógicas dentro de chamada composta. Uma leitura de ADR e um search+get+evidence
  não contam como o mesmo número de passos. Preservar logs para conferir a regra.
- Contar separadamente invocações do agente, juiz fake e provider do produto.
  Zero provider do produto só é observado se o caminho executado/log confirma;
  `calls` do fake não é chamada faturada. Configuração offline sozinha não é log.
- Tokens estimados = teto de caracteres Unicode observados / 4, por mensagem;
  somar entradas/saídas efetivamente registradas, incluindo retornos de ferramentas
  quando visíveis. Não somar bytes UTF-8 como caracteres. Sem acesso a histórico
  reenviado/system prompts, rotular como estimativa parcial, não consumo total.
  Não converter em preço, faturamento ou eficiência do modelo.
- Intervenções: guardar motivo, texto, instante e etapa de transporte/conteúdo.
  Intervenção que revela resposta/oráculo invalida a comparação daquela rodada;
  manter o artefato. Correção de transporte entra no resultado assistido, sem
  atribuir resolução autônoma ao agente. Lista vazia só após observar os logs.
- Tempo humano ativo: `null`/não medido. Tempo de máquina, espera, contador da UI,
  número de cliques de script e ações do agente não substituem essa medição.

Publicar por condição: pontos/24, tarefas corretas/6, erros críticos/6, faltas de
evidência/6, timeouts, intervenções, passos e tempos por tarefa. Comparar X−A e
A−N por tarefa; não juntar setup, latência de `build_pack` e duração do agente.
Menos passos com correção pior não é ganho. Sem dados de um par, delta = não
medido. Com seis tarefas dependentes e um agente/condição, não produzir taxa
populacional, intervalo de confiança ou conclusão estatística de superioridade.

## Estado desta sessão: executado versus pendente

| Item | Estado em 04/10/2026 |
| --- | --- |
| Leitura de fontes primárias, contratos, fixtures e relatos anteriores | Executada; fundamenta o protocolo, não novas respostas de sujeitos |
| Corpus, seis perguntas, condições, oráculo e métricas | Pré-registrados neste documento |
| Montagem do corpus ampliado e equivalência A↔X em runtime | Pendente |
| Três agentes novos / 18 respostas | Não executados: esta sessão não expõe `functions.task` nem ferramenta equivalente de lançamento isolado |
| Pontuação e tempos/contagens comparáveis desta rodada | Não medidos |
| Provider real do Xemnas ou chamada externa de modelo para avaliação | Não executados |
| Testes fake do juiz / testes de produção nesta entrega | Inspecionados, não executados; validação do código ocorre na sessão proprietária |
| Tempo humano ativo, custo monetário, preferência de humanos | Não medidos |

Não foram usadas as ferramentas de contexto deste projeto como material do
sujeito nem como oráculo: isso contaminaria a condição sem memória. Não lançar
agentes novos lendo este documento integralmente; ele contém as respostas.

**Conclusão atual:** há um protocolo operacional reproduzível, mas nenhuma prova
nova de ganho de correção ou produtividade. O bloqueio imediato é disponibilizar
lançamento com contexto novo e preparar os estados equivalentes. Depois disso,
esta rodada pode medir recuperação/correção por agentes sem provider do produto.
Demonstrar produtividade humana exige voluntários, tarefas de desenvolvimento
equivalentes e medição separada de criação, revisão, manutenção e retomada; essa
etapa permanece pendente e não pode ser inferida dos testes fake.
