# Resultado do piloto de recuperação N/A/X

Data: 04/10/2026.
Status: piloto executado; síntese semântica provisória do coordenador, sem
pontuação independente por campo e sem prova de produtividade humana.

**Resultado principal:** as três condições recuperaram os encaminhamentos das
seis tarefas na síntese conferida pelo coordenador. X não demonstrou vantagem
sobre ADR/documentação: foram relatadas 32 chamadas de ferramenta em X, 24 em A
e 26 em N. Essas contagens são agregadas por lote, não passos ou tempos por
tarefa. Não houve medição de tempo humano ativo.

## Pergunta e condições

O [protocolo de correção e custo de recuperação](experimento-produtividade-assertividade.md)
define seis tarefas sintéticas pareadas, com fontes brutas disponíveis a todos:

- **N:** fontes brutas, sem memória organizada.
- **A:** mesmas fontes e `memoria.md`, com ADRs, observação descritiva e histórico.
- **X:** mesmas fontes e snapshots exportados de recuperação do Xemnas local,
  em `lookup/`; esta execução é denominada **EXPORTSNAPSHOT**.

Um sujeito por condição começou em sessão nova de agente geral. Cada sessão
recebeu um lote inicial T1–T3 e foi retomada para T4–T6. São três sujeitos,
seis invocações do agente e 18 respostas a tarefas, não 18 sujeitos independentes.
As retomadas preservam a dependência entre as respostas de cada sujeito.

| Condição | Sessão registrada | Invocações |
| --- | --- | --- |
| N | `ses_ef7246cccffeBTW9jJ05wf2Qd2` | Uma nova e uma retomada |
| A | `ses_ef7246cbaffeptjYqcEieKJ10Z` | Uma nova e uma retomada |
| X | `ses_ef7246cabffe0c1QVtQhTIoLYD` | Uma nova e uma retomada |

O lançamento usou `functions.Task` no harness executor. A identificação
específica do modelo/versão dos sujeitos não foi disponibilizada no registro
consolidado; não atribuir a rodada a Luna nem ao modelo do coordenador.
Raciocínio, cache do provedor, tokens faturados e custo monetário não são conhecidos.

## Preparação e desvios do protocolo

O corpus foi preparado fora do checkout, com checkpoints separados `initial/`,
`updated/` e `unknown/` para N, A e X. Os arquivos comuns de cada checkpoint
foram preparados com os mesmos bytes nas três condições. O registro do
observador contém hashes SHA-256, tamanhos e metadados de preparação.

Em `initial/`, o manifesto declara `serde = "1"`. Em `updated/`, declara
`serde = "2"`. Em `unknown/`, `servico/Cargo.toml` é um diretório, impedindo uma
leitura válida de manifesto; E5 conserva a última leitura completa de requisito
`"2"` e registra falha de revalidação. Isso não simula remoção comprovada.
Os horários de E5 são marcadores do cenário, não cronômetros dos agentes.

| Aspecto | Execução observada / limite |
| --- | --- |
| Transporte X | Snapshots de busca, detalhe, exportação, contexto de arquivo e histórico; não API/MCP ao vivo |
| Pré-registro | `observer/preregistration.json` declara EXPORTSNAPSHOT e o desvio antes do lançamento |
| Turnos | Dois lotes de três tarefas, em vez de seis turnos individuais |
| Temporalidade | Raízes de checkpoint já preparadas, em vez de alterar uma única raiz entre turnos |
| A↔X | Textos de E1/E2/E3 preservados nos ADRs e nos motivos; E5 e histórico comuns; formatos e navegação diferentes |
| Adoção em X | Extração fake e confirmação laboratorial por script; não revisão ou confirmação humana |
| Preparação runtime | Preparação registrou uma validação de teste; não substitui execução dos sujeitos nem mede produtividade |
| Ferramentas | Leitura/busca local; X também relatou uma releitura de JSON via shell após truncamento |
| Limites por tarefa | O protocolo previa 20 chamadas e 8 minutos; os agregados não permitem verificar cumprimento por tarefa |

Os materiais organizados de A e X contêm o conhecimento equivalente, não a mesma
interface. N também pode recuperar os motivos de E1/E2: não foi privado da
verdade para favorecer as condições com memória.

O pré-registro ainda contém `subjects: "not_run"`: registra a preparação antes
dos sujeitos, não o estado final da rodada. Analogamente, o documento do
protocolo registra sua própria entrega anterior como não executada. Este relato
registra a execução posterior com os desvios acima.

## O que as respostas sustentaram

A tabela abaixo é uma **síntese**, não transcrição literal das respostas nem
placar C/J/E/L. O coordenador informou ter as respostas brutas e conferiu os seis
encaminhamentos comuns. Esta entrega dispõe dessa síntese e dos artefatos de
preparação; não reavalia independentemente cada citação e cada ferramenta aberta.

| Tarefa | Encaminhamento recuperado em N, A e X | Suporte do cenário |
| --- | --- | --- |
| T1 — persistência | Manter SQLite embutido para desktop individual offline; PostgreSQL foi substituído junto com a premissa de fila compartilhada | E1: `fontes/e1-persistencia.txt`; E0 sozinho não explica o motivo |
| T2 — busca | FTS5 lexical nesta versão; não prometer sinônimos nem busca semântica aprovada | E2: `fontes/e2-busca.txt` |
| T3 — contadores | Preservar a delegação cruzada local; não autorizar troca só para alinhar nomes; não anunciar teste aprovado ou autoridade dos mantenedores | E3: `fontes/e3-contadores.txt`; escolha local do avaliador, não regra RelayDesk; compilação/teste bloqueados |
| T4 — declaração | Requisito serde `"2"`, dependência runtime; declaração não prova versão instalada, uso em produção ou obrigação normativa | E5: manifesto em `updated/` e `fontes/e5-verificacao.txt` |
| T5 — lacunas | Retenção não definida; lavanda é demonstração, não cor obrigatória aprovada | Consulta ao conjunto comum e E4: `fontes/e4-terminal.txt` |
| T6 — falha de leitura | Estado atual desconhecido; última leitura completa verificou requisito `"2"`; falha não prova remoção; é necessária nova leitura válida | E5 em `unknown/`, com histórico de 10:00 e falha de 10:05 |

**Leitura provisória:** seis de seis encaminhamentos semânticos compatíveis com
o oráculo em cada condição, segundo a síntese do coordenador. Nenhum erro crítico
foi relatado nessa síntese. Isso não equivale a seis tarefas integralmente
corretas pela rubrica: ela também exige justificar, demonstrar a evidência
efetivamente consultada e preservar todos os limites em cada resposta.

| Medida de correção | Estado |
| --- | --- |
| Encaminhamentos compatíveis, segundo síntese | N 6/6; A 6/6; X 6/6, provisórios |
| Pontos C/J/E/L, máximo 24 por condição | Não atribuídos nesta entrega |
| Tarefas integralmente corretas pela rubrica | Pendente de conferência por campo das respostas e logs |
| Erros críticos / faltas de evidência auditados | Não quantificados independentemente |
| Avaliação anonimizada antes de revelar condição/custo | Não executada nesta entrega |

Não apresentar **24/24**, avaliação cega ou concordância entre avaliadores como
resultados observados. Uma avaliação posterior poderá conferir os logs brutos
por campo; uma sessão separada, sozinha, não prova independência de modelo.

## Chamadas de ferramenta relatadas

Contagens declaradas pelos sujeitos, consolidadas pelo coordenador. Cada lote
contém três tarefas; não há decomposição confiável por tarefa. Leitura de
diretório e tentativa de ler o manifesto-diretório não são leitura de arquivo.

| Condição | Lote inicial T1–T3 | Lote retomado T4–T6 | Total |
| --- | --- | --- | --- |
| N | 12: 10 leituras de arquivo + 2 de diretório | 14: 11 leituras de arquivo + 2 inventários + 1 leitura do manifesto-diretório | 26 |
| A | 6: 4 leituras de arquivo + 2 de diretório | 18: 13 leituras de arquivo + 4 de diretório + 1 glob | 24 |
| X | 13: 12 leituras + 1 glob | 19: 15 leituras + 2 grep + 1 glob + 1 shell para reler JSON truncado | 32 |

Diferenças agregadas de chamadas: **X−A = +8**, **A−N = −2** e **X−N = +6**.
São diferenças descritivas desta rodada, sem significância estatística. As
categorias de ferramenta não são idênticas e chamadas não equivalem
necessariamente a ações lógicas de recuperação, volume de contexto ou esforço.
Não dividir os totais por seis e publicar o quociente como custo observado de
cada tarefa. Também não descontar a releitura de X por ter sido inconveniente.

Nesta apresentação de snapshots, X navegou por mais artefatos e relatou mais
chamadas que A. O resultado **não sustenta a hipótese exploratória de menos
chamadas em X**. Tampouco mede o custo de busca/injeção ao vivo: outro transporte
pode produzir outro resultado e precisará de uma nova rodada.

## Tempo, chamadas de modelo e intervenções

| Medida | Registro disponível |
| --- | --- |
| Tempo de máquina por tarefa (`machine_wall_ms`) | `null`: sem fronteiras comparáveis de início/fim |
| Tempo humano ativo (`human_active_time_ms`) | `null`: não medido |
| Tempo/custo de criação, revisão e manutenção de memória | Não medido |
| Invocações do agente no harness | 6: três sessões novas e três retomadas |
| Provider real do produto Xemnas | 0 informado na preparação offline; sem execução de provider real do produto na rodada |
| Extração/adoção laboratorial | Fake/script, não inteligência semântica real nem trabalho humano medido |
| Tokens/caracteres efetivamente consumidos | Não consolidados; tamanhos dos arquivos são bytes, não consumo do modelo |
| Tokens faturados / preço | Desconhecidos |
| Timeouts e falhas de transporte | Nenhum relatado; sem auditoria completa de eventos por tarefa |
| Intervenções revelando respostas | Nenhuma relatada |
| Releitura via shell em X | Relatada; incluída nas 32 chamadas |

**Sem provider do produto não significa sem modelo:** as seis invocações dos
sujeitos pelo harness utilizaram agentes de modelo. Não chamar o experimento de
“zero IA”, nem inferir ausência de faturamento. Tempo de máquina e chamadas do
agente também não substituem tempo humano ativo.

## Rastro de artefatos e possibilidade de auditoria

Raiz local da rodada, fora do repositório:

```text
%LOCALAPPDATA%\Temp\opencode\xemnas-comparison-20261004\
```

| Caminho relativo à raiz | Papel |
| --- | --- |
| `N/initial/`, `N/updated/`, `N/unknown/` | Fontes brutas de cada checkpoint |
| `A/initial/`, `A/updated/`, `A/unknown/` | Fontes comuns e `memoria.md` |
| `X/initial/`, `X/updated/`, `X/unknown/` | Fontes comuns e `lookup/` com snapshots |
| `X/<checkpoint>/lookup/INDEX.md` | Entrada para os artefatos de recuperação |
| `X/<checkpoint>/lookup/search-*.json` | Respostas de busca exportadas |
| `X/<checkpoint>/lookup/decision-*.txt`, `export-*.json` | Detalhes e exportações de decisões |
| `X/<checkpoint>/lookup/file-context.json`, `observations-history.json` | Contexto e histórico descritivo |
| `observer/preregistration.json` | Desvio pré-registrado, adoção laboratorial e ausência de claim de produtividade |
| `observer/metadata.json` | Hashes, tamanhos, equivalência e cobertura da preparação |
| `observer/lab.sqlite`, `last-manifest.toml`, `reader-project/`, `avaliador-local-ripgrep/` | Banco e materiais do observador; não material autorizado aos sujeitos |

A raiz também contém `failed-attempt-*`, tentativas de preparação. Não são
sujeitos adicionais nem repetições pontuadas da comparação. Não excluir esses
artefatos ao reconstituir a preparação.

Os arquivos temporários não estão incorporados a este commit e podem deixar de
existir. Os IDs de sessão localizam as execuções no harness, mas não substituem
um arquivo versionado de prompts, respostas e chamadas de ferramenta. O relato
não afirma que a auditoria independente completa já seja reproduzível apenas
com este Markdown. Para pontuar, será preciso preservar e sanitizar esse rastro,
incluindo suporte aberto, textos integrais e eventos, antes de atribuir C/J/E/L.

## Conclusão e próximo experimento

O piloto é compatível com recuperação correta dos seis encaminhamentos nas três
condições, inclusive limites de autoridade, declaração versus instalação e falha
de leitura versus remoção. N conseguiu usar as fontes brutas. X não demonstrou
ganho de correção sobre A, e seu transporte por snapshots exigiu mais chamadas
relatadas. Não há prova de produtividade de desenvolvimento, economia de tempo
humano ou menor custo total de manutenção de decisões.

Próximos passos recomendados, ainda não executados:

1. Preservar os logs e conferir cada campo da rubrica contra o oráculo, com
   identificação das limitações do avaliador e sem transformar a síntese em nota.
2. Investigar revisão por exceção/duplicação e recuperação progressiva: mostrar
   primeiro o mínimo útil e abrir detalhes/evidências sob demanda. São hipóteses
   para reduzir custo, não benefícios comprovados nem mudanças implementadas.
3. Repetir com busca/injeção ao vivo, seis turnos e relógios por tarefa, mantendo
   fontes equivalentes, checkpoints comuns e registro de intervenções.
4. Fazer um experimento humano de desenvolvimento/retomada que meça separadamente
   criação, confirmação, correção, manutenção e uso da memória, comparando ADRs
   e Xemnas. O ganho líquido precisa incluir esses custos, não apenas recuperação.

Validação desta entrega documental: conferência do protocolo, dos caminhos,
pré-registro e materiais de checkpoint; revisão das somas e do diff. Não houve
nova rodada de agentes, avaliação independente ou execução de testes do produto
para elaborar este relato. A falta de logs integrais por campo e de cronômetros
é uma limitação da evidência disponível, não um resultado positivo.
