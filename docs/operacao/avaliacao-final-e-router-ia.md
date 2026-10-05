# Avaliação descritiva completa e router IA opcional

Data: 04/10/2026. Status: implementado e testado em cenários controlados; qualidade
real de modelo e produtividade humana ainda não comprovadas.

## Visão visual

Abra [visao-projeto.html](visao-projeto.html) no navegador. Documento local, sem
CDN/provider, com matriz de confiança, fluxo, métricas, evidências e notas que podem
ser baixadas em Markdown. Não é telemetria ao vivo nem tela nova do produto.
HTML/JavaScript, fontes e links foram verificados; inspeção visual de navegador
não foi executada nesta entrega.

## Avaliação descritiva

`observations_evaluation.rs` executou 16/16 cenários em cinco testes normais e um
relatório ignored. Exerceu manifests/categorias, seleção por arquivo sem grafo,
negativos, mudanças/remoções, fonte incerta, quotas, correções por sessão,
isolamento, histórico/prioridade normativa e cache após reopen.

Fatos exigidos recuperados: 11/11. Itens entregues pertinentes segundo o oráculo:
11/14 (78,57%). Negativos limpos: 3/3. Não são probabilidades globais de acerto,
qualidade de resposta de LLM ou prova de produtividade. A busca normativa mantém
baseline anterior 17/26 precisão, 17/18 recall e 2/15 negativos contaminados.

30 amostras locais por escopo, microssegundos (p50/p95): refresh cold 4840/6843;
warm/reopen 5820/7021; build_pack 474/649; render 18/32. Warm inclui reabertura;
não implica aceleração em toda execução. Zero provider no percurso descritivo.

Targets anteriormente bloqueados: extraction 5 e jobs 12 passaram com relinks
isolados, respectivamente nas tentativas 1 e 3. SAC permaneceu ativo; não se
insistiu indefinidamente no mesmo hash nem se usaram flags globais.

## Juiz de relevância opcional

Primeira consulta retorna baseline imediatamente, sem HTTP. Com novo consentimento
`context_routing`, ambiguidades de até seis candidatos já existentes podem gerar
job remoto; resultado validado fica em cache por até uma hora. Cache pode filtrar
itens tópicos fracos, não criar memória, remover normas fixas, reordenar itens ou
preencher o espaço liberado com outros registros. Pedidos com arquivos, edit,
file-only e históricos permanecem determinísticos neste slice.

Limites: dois pendentes por Project, oito solicitações lógicas/dia, cooldown de
60 segundos e 128 entradas. Uma conclusão lógica, sem reparo de JSON; retries
existentes podem produzir mais tentativas HTTP. Contagem física está indisponível,
não igual a um. Tokens são estimados, não faturamento. Tarefa protegida temporária
é apagada no término/recuperação; auditoria não conserva prompts completos.

Saída é allowlist de IDs/revisões e relevant/irrelevant/abstain. Código confere
autoridade, snapshot semântico, consentimento, dirty e CAS. Recheck idêntico não
invalida o cache. Jobs têm ownership exato; expirados podem ser renovados e cleanup
persiste nos retornos antecipados. Migration28 mantém schema original; 29 adiciona
ownership forward-only. Cache legado compatível pode ser reconhecido/rekeyado sem
nova chamada; trabalho legado sem ownership falha fechado.

## Evidência e limites

11 testes de routing passaram, incluindo lookup real de cache legado pós-upgrade
com uma chamada lógica total. Fmt, check desktop e Clippy dos quatro crates
all-targets passaram. Testes usam fakes/loopback: nenhum provider externo ou
keychain pessoal foi usado. Review independente encontrou quatro problemas de
cache/cleanup/ownership e um de reaproveitamento legado; todos foram corrigidos.
Review final independente confirmou o fechamento do reaproveitamento legado,
sem P1/P2 pendentes no delta examinado.
Teste separado de upgrade voltou a ser bloqueado por SAC na última rodada;
cenário equivalente dentro de routing executou. Não afirmar suíte única verde.

Consentimento ampliado muda o hash e exige autorização humana nova antes de
habilitar a finalidade. Não configurar credenciais nem autorizar envio pelo usuário.

## Produtividade e assertividade

Assertividade foi medida contra oráculos limitados. Não foi executado experimento
comparativo de tarefas com sessões novas ou participantes humanos. O
[protocolo pareado](experimento-produtividade-assertividade.md) define seis tarefas
e controles sem memória organizada/ADR/Xemnas, com conhecimento comparável.

Para comprovar valor, medir resultado correto, restrições omitidas, tempo ativo,
intervenções, retrabalho e custo de construir/manter memória; separar espera e
tokens estimados. Critérios ficam fixados antes das tarefas reservadas. Fake judge
prova infraestrutura, não assertividade do modelo. Não publicar ganho percentual
de produtividade sem essa execução.
