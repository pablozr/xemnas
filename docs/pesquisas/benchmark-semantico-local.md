# Benchmark semântico local: E5-small, EmbeddingGemma Q4, SQLite e LanceDB

**Data:** 2026-10-01; execução entre 2026-09-30 e 2026-10-01, horário de Brasília.
**Pergunta:** que qualidade, latência e memória esses modelos e índices oferecem
para o Xemnas local first, mirando 8 GB e CPU, sem worker Python?
**Status:** Aberta. Experimento concluído e resultados auditáveis; seleção da
stack do produto e validação nativa Windows na máquina mínima continuam pendentes.

## Leitura dos resultados

Para o índice inicial, os dados favorecem **SQLite + FTS5 + sqlite-vec**:
menor memória, integração simples e boa latência na base pequena. LanceDB
embedded tem uma alternativa interessante com ANN em escala; exige considerar
recall, memória, disco e a dificuldade de integração da release testada.

**E5-small favorece velocidade e inicialização. EmbeddingGemma Q4 favorece
memória e recuperação de contexto.** Ambos executaram inteiramente em CPU.
O arranjo de inferência e armazenamento testado não usa API paga nem servidor
de hospedagem: o trabalho e os dados ficam na máquina. Docker é o ambiente
deste laboratório; isso não determina o empacotamento do app.

Dois resultados restringem a proposta anterior de automação: similaridade não
verifica causalidade ou a escolha correta, e a fusão RRF com pesos iguais
**piorou** a qualidade da fixture. Ela continua sendo uma configuração a avaliar,
não uma política aprovada. Os contextos recuperados devem manter evidências,
origem e validação antes de fundamentar relações entre decisões.

## Ambiente e método efetivamente executados

- Host: Windows 11 Pro 26200, Ryzen 5 5600G, 6 núcleos/12 threads, aproximadamente
  32 GiB físicos, plano Equilibrado e sete outros containers ativos.
- Medição: Linux/WSL2 em Docker, **8 GiB de limite**, **swap zero**, quota de
  quatro CPUs equivalentes, CPU como único provider ONNX e rede desligada.
  Não houve isolamento de núcleos físicos nem suspensão dos demais serviços.
- Modelos em bind NTFS do D:; bancos em volume Linux ext4 nativo. Inicialização
  inclui leitura do cache, tokenizer e preparo da sessão ONNX. É processo novo
  com arquivos já locais, não um teste de cache frio do sistema operacional.
- FastEmbed Rust 7.1.0; ONNX Runtime CPU 1.30.0 dinâmico; sqlite-vec 0.1.9;
  rusqlite 0.40.2 bundled; LanceDB 0.39.0 **com patch de compilação embedded**.
  Lockfile, hashes e patch estão preservados. Build release sem LTO, com
  `codegen-units=256`, Rust 1.98.1.
- Prompts oficiais, vetores normalizados, dimensão original e limite comum de
  512 tokens. Não houve truncamento das entradas da fixture ou do Belebele.
- 48 decisões fictícias, 24 consultas PT/EN e julgamentos binários congelados.
  Duas decisões relevantes por consulta e exemplos difíceis de alternativas
  opostas. O teste público é o Belebele convertido para retrieval pelo MTEB:
  488 passagens/900 consultas por idioma, PT e EN completos.
- Escala de 1.000/10.000/100.000 linhas: textos repetidos e perturbações
  determinísticas dos vetores reais. **Não são cem mil decisões reais**, nem
  uma avaliação semântica nessa escala. Isso cria agrupamentos artificiais e
  pode alterar a dificuldade do ANN e o comportamento das listas de FTS.
- 8 casos de modelos, 48 de armazenamento, 12 pipelines completos e 4 de
  qualidade pública. As estatísticas abaixo são medianas dos p50/p95 de três
  processos por configuração principal, sem juntar amostras como se fossem
  independentes. Cada processo tem 120 consultas curtas ou 240 buscas por
  operação. Uma thread e cada pipeline têm **um processo**; cada perfil longo
  tem 20 amostras/processo. Percentil por nearest rank.

O protocolo, suas fontes primárias e os comandos de reprodução estão em
[metodologia-benchmark-semantico.md](metodologia-benchmark-semantico.md).
A validação das formas, amostras, IDs e métricas passou. Houve concordância
de **100% dos conjuntos top-10 exatos** entre SQLite e LanceDB em todas as
576 comparações consulta/configuração; isso não afirma igualdade de scores.

## Modelos: consulta curta, inicialização e memória

Tempos em milissegundos, exceto inicialização em segundos. RAM é pico residente
do processo Linux, não memória total da máquina. A auditoria do tokenizer ocorre
antes de carregar o modelo, mas também entra no contador de pico. As medições
de pipeline mais adiante não executam essa auditoria separada.

| Modelo | Dimensão | Arquivos locais MiB | Inicialização s | Consulta p50 ms | Consulta p95 ms | RAM após consultas curtas MiB | Pico fixture/batch longo MiB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| E5-small | 384 | 464,8 | 6,05 | 7,36 | 11,30 | 1036 | 1437 |
| Gemma Q4 | 768 | 208,6 | 30,05 | 28,50 | 57,00 | 556 | 697 |

E5-small: p50 entre 7,35 e 9,13 ms; p95 entre 9,60 e 13,43 ms. O pico no corpus público foi 1204 MiB.

Gemma Q4: p50 entre 25,32 e 31,86 ms; p95 entre 36,44 e 125,33 ms. O pico no corpus público foi 970 MiB.

O p95 do Gemma variou bastante entre processos; não é uma garantia de latência
no hardware mínimo. O limite de CPU e a concorrência do host fazem parte destas
condições. A inicialização de aproximadamente 6/30 segundos torna a reutilização
de sessão e o trabalho fora da thread da UI escolhas necessárias para uma busca
responsiva. Isso não é o tempo de aprovação de uma decisão no app, que não foi medido.

### Uma thread versus quatro

| Modelo | Uma thread p50/p95 ms | Quatro threads p50/p95 ms |
| --- | --- | --- |
| E5-small | 12,04 / 14,75 | 7,36 / 11,30 |
| Gemma Q4 | 50,31 / 70,43 | 28,50 / 57,00 |

O caso de uma thread não foi repetido em três processos. Ele mostra viabilidade
com menos paralelismo; não mede disputa com a UI nem fornece uma curva completa
para CPUs distintas.

### Comprimento da entrada

Entradas sintéticas próximas de 128/256/512 tokens, com tamanho real contado
pelo tokenizer de cada modelo. Os textos têm comprimentos diferentes para
aproximar o mesmo orçamento de tokens. O p95 tem apenas 20 amostras/processo.

| Modelo | Alvo tokens | Tokens reais | p50 ms | p95 ms |
| --- | --- | --- | --- | --- |
| E5-small | 128 | 120 | 30,39 | 66,57 |
| E5-small | 256 | 253 | 61,28 | 105,40 |
| E5-small | 512 | 500 | 175,51 | 262,58 |
| Gemma Q4 | 128 | 118 | 111,80 | 151,34 |
| Gemma Q4 | 256 | 244 | 217,26 | 265,41 |
| Gemma Q4 | 512 | 496 | 518,58 | 660,23 |

### Indexação dos textos: batch 8

O primeiro corpus tem 48 decisões curtas. Cada processo mede três execuções
do corpus; a tabela mostra mediana das médias e o throughput correspondente,
sem p95 de três observações. O segundo corpus contém 488 passagens públicas,
com comprimentos variados e padding por lote; uma execução por idioma/modelo.

| Modelo | 48 decisões: média ms | Decisões/s nessa média | 488 passagens PT: segundos | Passagens/s PT | 488 passagens EN: segundos |
| --- | --- | --- | --- | --- | --- |
| E5-small | 482,50 | 99,48 | 36,32 | 13,44 | 32,15 |
| Gemma Q4 | 2358,72 | 20,35 | 196,98 | 2,48 | 135,68 |

Compilação, download e inicialização não entram nesses tempos de encoding.
Eles são testes de textos diretamente entregues ao modelo, **não ingestão de
PDF/livro**. Padding e distribuição de comprimentos impedem transferir esse
throughput diretamente para uma futura biblioteca. Os tempos de batch 1 e
do stress de oito entradas longas estão nos arquivos de cada processo.

## Qualidade: relevância e escolhas inadequadas

Recall@10 conta relevantes recuperados; MRR@10 e nDCG@10 valorizam posições
mais altas. MRR/nDCG são valores normalizados de 0 a 1, não taxas de acerto.
Todos os rankings e julgamentos foram preservados para auditoria.

### Belebele PT/EN: recuperação vetorial exata

Um relevante por consulta neste conjunto; 900 consultas/488 passagens por
idioma. São dois testes monolíngues paralelos, não o leaderboard MTEB inteiro
nem um teste de código. O limite comum é 512 tokens, sem truncamento observado.

| Modelo | Idioma | Recall@10 % | MRR@10 | nDCG@10 | Top-1 relevante % |
| --- | --- | --- | --- | --- | --- |
| E5-small | PT | 96,56 | 0,888 | 0,906 | 84,67 |
| E5-small | EN | 98,56 | 0,937 | 0,949 | 90,78 |
| Gemma Q4 | PT | 98,44 | 0,935 | 0,947 | 90,56 |
| Gemma Q4 | EN | 99,33 | 0,957 | 0,966 | 93,44 |

Gemma foi melhor nos dois idiomas neste corpus. E5 também teve boa recuperação
geral, com muito menos tempo de encoding. Esses resultados não estabelecem
superioridade em todos os projetos ou tipos de consulta.

### Decisões adversariais: 24 consultas, 48 decisões fictícias

Os qrels têm duas decisões relevantes por consulta, uma PT e uma EN com a
mesma escolha. O negativo difícil apresenta uma escolha oposta ou inadequada
do mesmo tema. Esta é uma fixture editorial, não uma estimativa da taxa de
erro no repositório real do usuário.

| Modelo | Recall@10 % | MRR@10 | nDCG@10 | Top-1 relevante % | Top-1 negativo difícil % |
| --- | --- | --- | --- | --- | --- |
| E5-small | 64,58 | 0,634 | 0,545 | 45,83 | 8,33 |
| Gemma Q4 | 91,67 | 0,727 | 0,738 | 54,17 | 25,00 |

Gemma recuperou mais relevantes no top-10, mas colocou negativos difíceis
primeiro em **6/24 consultas**, contra **2/24 no E5**. Por exemplo, a pergunta
fictícia sobre rastrear a origem de uma aresta trouxe primeiro o texto fictício
que propõe transformar similaridade acima de 0,8 em dependência causal sem
evidência. Esse 0,8 faz parte do texto do exemplo; não é um score medido aqui.

A conclusão prática é usar embeddings para **recuperar candidatos e contexto**.
Relações `depende_de`, `substitui` ou uma escolha correta precisam de evidência
e validação próprias. Os vínculos comprováveis por arquivos realmente editados
continuam pertencendo à captura estruturada de evidência, não a esse ranking.

### FTS e RRF: a configuração testada degradou a qualidade

Mesmo RRF nas duas engines: top-20 vetorial e top-20 lexical, pesos iguais,
constante 60, top-10 final e desempate por ID. O texto FTS não contém prompts
dos modelos; OR entre termos alfanuméricos, com escape no SQLite. Stemming e
stopwords em inglês desabilitados no Lance; `unicode61 remove_diacritics 2`
no FTS5. Tokenizers e variantes BM25 não são idênticos.

| Modelo | Engine | Vetorial nDCG@10 | FTS nDCG@10 | RRF nDCG@10 | RRF top-1 negativo % |
| --- | --- | --- | --- | --- | --- |
| E5-small | SQLite | 0,545 | 0,268 | 0,377 | 25,00 |
| E5-small | LanceDB | 0,545 | 0,259 | 0,372 | 20,83 |
| Gemma Q4 | SQLite | 0,738 | 0,268 | 0,450 | 33,33 |
| Gemma Q4 | LanceDB | 0,738 | 0,259 | 0,450 | 33,33 |

Não há ganho de qualidade automático por adicionar FTS/RRF. O lexical fraco
arrastou o ranking denso nesta fixture de paráfrases e alternativas opostas.
Ela não cobre adequadamente consultas literais por símbolos/caminhos de código;
por isso o resultado não prova inutilidade de FTS ou RRF em geral. **A fusão
uniforme testada não deve virar default do produto sem avaliação adicional**
com consultas reais, critérios de combinação e validação dos candidatos.

## Armazenamento: embeddings pré-calculados

Busca por cosseno, top-10; o híbrido materializa top-20 de cada modalidade e
aplica RRF. A busca vetorial abaixo é **exata** nos dois lados. Lance usa
`bypass_vector_index()` explicitamente, antes da criação do ANN nesta etapa.
Os mesmos IDs, vetores serializados e textos foram usados nas duas engines.

| Modelo | Linhas | Engine | Vetorial p50 ms | FTS p50 ms | Híbrido p50 ms | Híbrido p95 ms |
| --- | --- | --- | --- | --- | --- | --- |
| E5-small | 48 | SQLite | 0,46 | 0,11 | 0,61 | 0,85 |
| E5-small | 48 | LanceDB | 1,78 | 1,19 | 2,97 | 4,28 |
| E5-small | 1.000 | SQLite | 0,77 | 0,48 | 1,28 | 2,09 |
| E5-small | 1.000 | LanceDB | 2,35 | 1,33 | 3,72 | 5,50 |
| E5-small | 10.000 | SQLite | 7,37 | 4,00 | 11,43 | 18,14 |
| E5-small | 10.000 | LanceDB | 8,72 | 1,71 | 10,53 | 14,65 |
| E5-small | 100.000 | SQLite | 66,18 | 37,19 | 104,92 | 147,36 |
| E5-small | 100.000 | LanceDB | 128,61 | 2,63 | 131,28 | 184,12 |
| Gemma Q4 | 48 | SQLite | 1,16 | 0,14 | 1,33 | 1,97 |
| Gemma Q4 | 48 | LanceDB | 2,21 | 1,53 | 3,79 | 5,72 |
| Gemma Q4 | 1.000 | SQLite | 1,62 | 0,51 | 2,15 | 2,92 |
| Gemma Q4 | 1.000 | LanceDB | 2,98 | 1,33 | 4,25 | 6,40 |
| Gemma Q4 | 10.000 | SQLite | 14,81 | 3,94 | 18,73 | 28,08 |
| Gemma Q4 | 10.000 | LanceDB | 31,56 | 1,81 | 34,05 | 51,28 |
| Gemma Q4 | 100.000 | SQLite | 147,59 | 45,26 | 198,31 | 308,14 |
| Gemma Q4 | 100.000 | LanceDB | 199,64 | 2,68 | 207,04 | 285,96 |

### Construção, disco e memória antes do ANN

Construção inclui população e FTS; não inclui embeddings nem treinamento ANN.
SQLite: transação única, WAL, `synchronous=FULL`, cache de 32 MiB e checkpoint
TRUNCATE. Lance: criação local por reader em lotes de 1.024 linhas e FTS nativo,
durabilidade padrão do SDK. Os tempos não são um teste de fsync equivalente
entre implementações. O pico de RAM inclui construção e buscas exatas.

| Modelo | Linhas | Engine | Construção s | Banco MiB | Pico processo MiB |
| --- | --- | --- | --- | --- | --- |
| E5-small | 48 | SQLite | 0,05 | 1,58 | 27 |
| E5-small | 48 | LanceDB | 0,33 | 0,10 | 77 |
| E5-small | 1.000 | SQLite | 0,13 | 1,80 | 27 |
| E5-small | 1.000 | LanceDB | 0,38 | 1,50 | 101 |
| E5-small | 10.000 | SQLite | 0,47 | 17,41 | 45 |
| E5-small | 10.000 | LanceDB | 0,45 | 14,95 | 252 |
| E5-small | 100.000 | SQLite | 2,62 | 171,71 | 60 |
| E5-small | 100.000 | LanceDB | 0,98 | 149,16 | 530 |
| Gemma Q4 | 48 | SQLite | 0,06 | 3,08 | 30 |
| Gemma Q4 | 48 | LanceDB | 0,37 | 0,17 | 79 |
| Gemma Q4 | 1.000 | SQLite | 0,15 | 3,30 | 31 |
| Gemma Q4 | 1.000 | LanceDB | 0,35 | 2,96 | 131 |
| Gemma Q4 | 10.000 | SQLite | 0,41 | 32,43 | 61 |
| Gemma Q4 | 10.000 | LanceDB | 0,55 | 29,60 | 303 |
| Gemma Q4 | 100.000 | SQLite | 5,38 | 318,91 | 62 |
| Gemma Q4 | 100.000 | LanceDB | 1,53 | 295,60 | 760 |

Em 100 mil linhas, SQLite teve menor memória e busca vetorial exata mais rápida
nestas configurações. Lance teve FTS muito mais rápido e armazenamento exato
menor. Em bases pequenas SQLite teve menos overhead. Não se deve escolher um
vencedor universal a partir de um corpus repetido e destes parâmetros.

Q4 quantiza **pesos do modelo**, não os embeddings persistidos. Ambos os índices
guardam float32; Gemma usa 768 dimensões e E5 384. Só os vetores de 100 mil linhas
ocupam aproximadamente 293/146 MiB respectivamente, antes dos demais dados.
Saídas Matryoshka menores de Gemma não foram testadas.

## Recuperação completa: texto → embedding → índices → RRF

Uma sessão carregada, consultas aquecidas, batch 1, quatro threads ONNX,
buscas sequenciais e top-10 materializado. **Um processo/120 amostras por caso**,
não três processos. A etapa Lance é exata, mesmo com um ANN já presente no banco.
O tempo não inclui inicialização, UI, fila, reranker ou resposta de LLM.

| Modelo | Linhas | Engine | Pipeline p50 ms | Pipeline p95 ms | Pico processo MiB |
| --- | --- | --- | --- | --- | --- |
| E5-small | 1.000 | SQLite | 12,36 | 22,75 | 1014 |
| E5-small | 1.000 | LanceDB | 13,36 | 22,76 | 1066 |
| E5-small | 10.000 | SQLite | 18,74 | 27,48 | 1013 |
| E5-small | 10.000 | LanceDB | 17,48 | 24,93 | 1221 |
| E5-small | 100.000 | SQLite | 112,78 | 164,86 | 1013 |
| E5-small | 100.000 | LanceDB | 95,45 | 149,30 | 1552 |
| Gemma Q4 | 1.000 | SQLite | 24,69 | 30,62 | 541 |
| Gemma Q4 | 1.000 | LanceDB | 30,49 | 39,39 | 546 |
| Gemma Q4 | 10.000 | SQLite | 52,84 | 96,57 | 540 |
| Gemma Q4 | 10.000 | LanceDB | 55,20 | 111,17 | 699 |
| Gemma Q4 | 100.000 | SQLite | 210,38 | 307,85 | 540 |
| Gemma Q4 | 100.000 | LanceDB | 189,43 | 238,31 | 1234 |

Esses tempos foram medidos inteiros: não são soma dos p95 das etapas. A presença
do modelo, os pools de threads, o cache e a execução em outro momento/processo
mudam o cenário. O resultado do pipeline não precisa ordenar as engines como
as medianas das buscas isoladas; a inversão em 100 mil não é uma garantia de
vantagem, pois o pipeline teve uma repetição por caso. A qualidade desse RRF
experimental continua sujeita à degradação mostrada acima.

## ANN LanceDB: velocidade acompanhada de fidelidade

IVF_FLAT, cosseno, partições `floor(sqrt(N))`, máximo de 50 iterações e
`sample_rate=256` padrão do SDK. Sem seed exposta nesta API. `nprobes` fixa
quantas partições consultar. O recall abaixo é a interseção dos top-10 por ID
com os top-10 exatos da própria engine; **não é relevância semântica**.

| Modelo | Linhas | nprobes | ANN p50 ms | ANN p95 ms | Recall vs exato % | Faixa recall entre processos % |
| --- | --- | --- | --- | --- | --- | --- |
| E5-small | 1.000 | 1 | 2,08 | 3,13 | 42,92 | 39,58–43,33 |
| E5-small | 1.000 | 4 | 1,95 | 2,45 | 71,25 | 69,17–74,17 |
| E5-small | 1.000 | 16 | 1,72 | 2,19 | 100,00 | 100,00–100,00 |
| E5-small | 10.000 | 1 | 1,74 | 2,41 | 42,50 | 41,67–44,58 |
| E5-small | 10.000 | 4 | 1,72 | 2,29 | 69,58 | 68,33–69,58 |
| E5-small | 10.000 | 16 | 2,05 | 2,61 | 97,08 | 90,83–98,33 |
| E5-small | 100.000 | 1 | 2,37 | 3,14 | 16,67 | 16,25–20,83 |
| E5-small | 100.000 | 4 | 2,18 | 3,50 | 52,50 | 48,75–53,33 |
| E5-small | 100.000 | 16 | 2,39 | 3,28 | 84,17 | 83,75–84,58 |
| Gemma Q4 | 1.000 | 1 | 1,62 | 2,08 | 66,25 | 60,83–70,83 |
| Gemma Q4 | 1.000 | 4 | 1,60 | 1,92 | 98,75 | 95,00–98,75 |
| Gemma Q4 | 1.000 | 16 | 2,01 | 2,63 | 100,00 | 100,00–100,00 |
| Gemma Q4 | 10.000 | 1 | 1,69 | 2,13 | 55,42 | 53,33–64,17 |
| Gemma Q4 | 10.000 | 4 | 1,74 | 2,20 | 98,75 | 96,67–99,17 |
| Gemma Q4 | 10.000 | 16 | 2,27 | 3,97 | 100,00 | 100,00–100,00 |
| Gemma Q4 | 100.000 | 1 | 2,84 | 4,38 | 23,75 | 22,92–29,58 |
| Gemma Q4 | 100.000 | 4 | 3,55 | 5,45 | 70,00 | 64,58–72,08 |
| Gemma Q4 | 100.000 | 16 | 4,45 | 9,24 | 100,00 | 100,00–100,00 |

| Modelo | Linhas | Partições | Construção ANN s | Banco com ANN MiB | Pico incluindo ANN MiB |
| --- | --- | --- | --- | --- | --- |
| E5-small | 1.000 | 31 | 0,07 | 3,01 | 112 |
| E5-small | 10.000 | 100 | 0,39 | 29,77 | 318 |
| E5-small | 100.000 | 316 | 7,24 | 296,32 | 886 |
| Gemma Q4 | 1.000 | 31 | 0,08 | 5,99 | 151 |
| Gemma Q4 | 10.000 | 100 | 0,73 | 59,22 | 470 |
| Gemma Q4 | 100.000 | 316 | 18,41 | 589,71 | 1425 |

Com 100 mil linhas e 16 probes, E5 recuperou aproximadamente 84% dos vizinhos
exatos; não é uma comparação de fidelidade equivalente a SQLite exhaustive.
Gemma obteve 100% nas 24 consultas e nas três construções, com cerca de 4,45 ms
de mediana. É um resultado promissor **neste corpus artificialmente agrupado**,
não prova de recall perfeito em uma base real. Muitos vizinhos têm textos
duplicados, e perder um ID vizinho não significa necessariamente perder contexto
semântico distinto. Novos dados precisam de uma curva própria de recall/latência.

IVF_FLAT guarda novamente vetores no índice: o disco com ANN ficou próximo do
dobro do armazenamento Lance exato. Em 100 mil, os picos de processo incluindo
construção/ANN foram aproximadamente 886 MiB para E5 e 1.425 MiB para Gemma,
**sem o modelo de embedding residente nesses processos**. Não foi medido o
pipeline completo com ANN; não se deve somar seu p50/p95 ao do modelo e apresentar
isso como latência observada.

## Integração, limitações e validação

O build nativo Windows foi bloqueado pelo Smart App Control ao carregar a DLL
de proc-macro `yoke_derive` (evento 3077). A configuração de segurança permaneceu
intacta. Estes resultados são **Linux CPU com cap de 8 GiB**, em host de 32 GiB;
não certificam Windows com 8 GB físicos, especialmente com SO/editor/app abertos.

Na preparação, sqlite-vec 0.1.10-alpha.4 referenciou um arquivo C ausente;
o experimento fixou a stable 0.1.9. O arquivo estático ONNX usado por default
não ligou no Debian Bookworm por símbolos de glibc/libstdc++ mais novos;
foi usado o runtime CPU oficial dinâmico, com checksum conferido. Protobuf
precisou de compiler e headers. Clippy/rustfmt foram adicionados à imagem.

LanceDB 0.39.0 referencia `Error::Http` sem condicionar os dois ramos de `job.rs`
à feature que define a variante. A feature `remote` ampliou as dependências e
sua tentativa foi interrompida. O experimento usa um tarball publicado com
SHA-256 conferido e patch versionado só nesses ramos de erro, sem alterar busca,
indexação ou persistência. O JSON identifica a release; `manifest.json` e o
protocolo identificam explicitamente o patch. Adoção no produto ainda precisa
selecionar uma release upstream corrigida ou decidir carregar essa correção.

Validação: `cargo fmt --all -- --check` na raiz e no workspace independente;
build release, três testes do harness e Clippy all-targets com `-D warnings`
no pacote alterado; 12 testes de `architecture`. LanceDB, dependência externa
com patch, emitiu warnings de código não usado; o Clippy do harness passou.
O script de resumo passou em todos os casos. Contadores após armazenamento
registraram zero OOM, zero eventos de limite de memória e swap zero; todas
as etapas públicas também terminaram normalmente. A quota CPU teve throttling
registrado e os valores refletem concorrência real do host.

O harness completo ficou com aproximadamente 338 MiB de executável, somando
as duas engines e os modelos suportados, mais cerca de 28 MiB do runtime ONNX.
Isso descreve este build, não o tamanho final de distribuição do Xemnas.
Não foram testados filtros por projeto, atualizações/exclusões, recuperação após
crash, concorrência de consultas, um corpus real de código, pipeline de PDF,
reranking, geração por LLM ou aprovação/materialização de relações do grafo.

## Artefatos e reprodução

- [Resumo validado](benchmarks/2026-10-01/summary.json): mediana, mínimo, máximo,
  valores por processo, métricas e concordância exata.
- [Manifesto](benchmarks/2026-10-01/manifest.json): revisão dos modelos, commit
  `7249e48` do harness, patch e inventário de arquivos com SHA-256.
- [Dados e resultados completos](benchmarks/2026-10-01/): fixture, qrels, rankings,
  vetores usados no armazenamento, amostras, GNU time, ambiente e hashes.
- [Validação e preparação](benchmarks/2026-10-01/validation/): testes, Clippy,
  bloqueio Windows e observações da preparação inicial anteriores ao harness
  final. `final-checks.log` inclui a tentativa sem Clippy; `clippy.log` registra
  a verificação bem-sucedida posterior.

Os dados derivados Belebele em `data/` preservam atribuição Meta/MTEB, revisão,
hash original e modificações em cada JSON e são distribuídos sob
[CC-BY-SA-4.0](https://creativecommons.org/licenses/by-sa/4.0/).
As decisões da fixture são fictícias. Pesos dos modelos, runtime, tarball do
SDK e bancos grandes não foram colocados no Git.

Para repetir, executar `tools/semantic-bench/run.ps1` no checkout deste commit,
com Docker, diretório temporário com espaço e rede somente na preparação;
depois `summarize.ps1` sobre o diretório retornado. O script fixa os modelos
E5 `614241f622f53c4eeff9890bdc4f31cfecc418b3` e Gemma
`5090578d9565bb06545b4552f76e6bc2c93e4a66`, e popula a cópia isolada do SDK.
O protocolo registra o override Cargo exigido também por testes/Clippy.

As decisões duradouras do produto ainda pertencem aos ADRs. Este relatório
mantém o experimento e suas limitações como pesquisa, sem alterar o app.
