# Inspiração: apps em GPUI e produtos do mercado

Data: 2026-10-02.

**Pergunta:** o que os melhores apps em GPUI e os produtos próximos ao xemnas
fazem que o nosso ainda não faz, e que mudanças visuais e ideias de produto
valem a pena daqui para frente?

**Status:** Aberta. É material de decisão: nada aqui é escopo aprovado. A
fila visual alimenta `acabamento-visual-revisao.md`; as ideias de produto
complementam `ideias-de-produto.md`.

## O que foi lido, e com que confiança

- **Lido direto:** o repositório do Zeron (`zeronsh/zeron`, clonado raso): o
  catálogo de movimento (`crates/ui/src/motion.rs`), `docs/theme-system.md`,
  `docs/sound-design`, `docs/research/sidebar-view-options.md`, e as capturas
  em `docs/screenshots` (paleta de comandos, barra lateral, medidor de
  contexto). Aqui vale o que está no código e nas imagens.
- **Lido por resumo de busca (confirmar nas fontes):** gpui-component
  (Longbridge), Hunk, Waku, Log4brains/adr-tools/Backstage, Mem0/Zep/Letta,
  Greptile/CodeRabbit/Dosu, Superhuman, Obsidian/Neo4j Bloom/Kumu,
  Langfuse/LangSmith, Raycast. Aqui vale o que a busca devolveu; nenhum
  desses foi instalado nem usado.
- **Limite:** o Zeron usa um fork do GPUI (fade por pixel, desfoque do fundo);
  o nosso é o Zed oficial. Cada item abaixo diz se cabe no nosso.

---

## Parte A. UI e UX em GPUI

### O que o Zeron faz bem (e vimos nas capturas)

1. **Paleta de comandos de produto.** Campo grande com o atalho no canto
   (`Ctrl K`), grupo "Ações" com ícone, depois o histórico em linhas de dois
   níveis (origem em cima, título embaixo, tempo à direita) e um rodapé fixo
   com as dicas `↑ ↓ Navegar · ↵ Abrir · esc Fechar`.
2. **Barra lateral com seções.** "Fixadas", "Sessões" e "Arquivadas",
   recolhíveis, com ponto de estado, selo da origem, tempo relativo curto
   (`17m`, `2h`, `1w`) e selos de número (`#413`). Um menu de "opções de
   visualização" (organizar por projeto ou em lista única; ordenar por
   prioridade, atualização ou manual) que persiste no escopo da barra.
3. **Medidor de contexto.** Um anel pequeno junto ao campo de escrita que vira
   vermelho perto do limite e abre um popover com "184000 / 200000 tokens,
   16000 restantes".
4. **Catálogo de movimento com números.** Entrada de 500 ms em ease-out
   exponencial com 4 px de subida; rápido 150 ms; menu 140 ms; diálogo 180 ms;
   recolher 180 ms; seta 200 ms; hover com **fade de 150 ms** (a cor
   interpola, não salta); redimensionar 200 ms com "quique" de 5 px ao bater
   na borda. Laço de marca (grade de pixels com onda de luz, 2,4 s) como
   carregamento e abertura com saída de 500 ms.
5. **Pausa em segundo plano.** A preferência de movimento tem três valores
   (sistema, ligado, desligado) e uma opção para **parar as animações quando
   a janela perde o foco**.
6. **Tema como sistema.** Variante completa por tema (claro e escuro
   escolhidos separadamente), **acento à parte, com contraste verificado**,
   preferência de superfície separada (tema, fosca, opaca), importação de
   temas do VS Code e uma passada que garante 4,5:1 para o texto principal e
   3:1 para o mudo sobre o pior fundo.
7. **Som de produto.** Quatro sinais curtos (concluído, pergunta, atenção,
   captura), gerados por síntese (sem amostras), com chave geral e escolha por
   tipo; os demais ficam como "audições" sem gatilho.
8. **Fade nas bordas da rolagem** (texto some por pixel), que depende do fork.

### O que o ecossistema GPUI acrescenta (por resumo)

- **gpui-component** traz 60 a 75 componentes: tabela virtual com colunas
  redimensionáveis e ordenação, lista virtual de altura variável, **dock**
  com painéis redimensionáveis e abas arrastáveis serializáveis, gráficos,
  notificações, Markdown e editor. Vale como referência de *desenho*; usar a
  biblioteca inteira pesaria na dependência (contra o pilar).
- **Hunk** (diff viewer + orquestrador): comparação lado a lado e revisão por
  *thread* de comentários ancorados ao trecho. Referência para a nossa
  evidência de código.
- **Waku** (app nativo para agentes de código): fila e "direcionar" mensagens
  enquanto o agente trabalha, estado local sem conta. Referência de posicionamento.
- **Loungy** (lançador): foco total no teclado e na busca.

### Onde o nosso app está, em relação a isso

Já temos: paleta com grupos, atalhos e busca sem acento; mascote e opening;
cinco temas e fundos; cascata, elevação, modo foco; listas virtuais; foco
visível e papéis de acessibilidade. Faltam, por impacto e custo:

| # | Mudança | Origem | Cabe no GPUI oficial? | Esforço |
| --- | --- | --- | --- | --- |
| V1 | **Pausar movimento com a janela inativa** e preferência de movimento (sistema, ligado, desligado) | Zeron | Sim (evento de ativação da janela + o relógio compartilhado que já temos) | Baixo; serve direto ao pilar |
| V2 | Rodapé de dicas e **fuzzy com recentes** na paleta; grupo "Ações" com ícone | Zeron | Sim | Baixo |
| V3 | **Hover com fade de 150 ms** nas linhas e botões (interpolar a cor em vez de trocar) | Zeron | Sim (já fazemos `hover_tint`; generalizar) | Baixo |
| V4 | **Tempo relativo curto** (`17 min`, `2 h`, `1 sem`) nas linhas de Revisão, Decisões e Projetos, com a data cheia no tooltip | Zeron | Sim | Baixo |
| V5 | **Anel de orçamento de contexto** com popover (usado/limite) no Contexto e junto ao mascote | Zeron | Sim (círculo por `PathBuilder`) | Médio |
| V6 | Lista de projetos com **seções recolhíveis** (Fixados, Recentes, Arquivados) e opções de visualização persistentes | Zeron | Sim | Médio |
| V7 | **Acento escolhido pela pessoa**, com verificação de contraste, separado do tema; claro e escuro escolhidos à parte | Zeron | Sim (nossos temas já são tabelas de paleta) | Médio |
| V8 | **Laço da marca** (grade em onda) como indicador de carga e esqueleto "pulsante" no relógio compartilhado | Zeron | Sim | Médio |
| V9 | **Desfazer** (toast com "Desfazer", tecla `Z`) em confirmar, rejeitar e adiar | Superhuman | Sim | Médio |
| V10 | **Painéis redimensionáveis** com largura persistente e quique na borda | Zeron, gpui-component | Sim | Médio |
| V11 | Modo **tabela** para "todas as decisões" (colunas ordenáveis, virtual) além da lista | gpui-component | Sim | Alto |
| V12 | **Sons opcionais** (concluído, atenção, captura), desligados por padrão, síntese sem amostras | Zeron | Sim (um canal de áudio; nova dependência, avaliar) | Médio |
| V13 | Fade de borda por pixel na rolagem | Zeron | **Não** (precisa do fork); em painéis opacos dá para sobrepor um degradê | Alto |
| V14 | Testes de interface headless com asserções de foco e acessibilidade | gpui-component | Sim (GPUI traz test-support) | Médio; protege tudo acima |

**Recomendação visual:** V1, V3, V4 e V2 num lote só (baixo custo, todos
reforçam "calmo e rápido"); depois V9, V6 e V7. V13 fica de fora enquanto não
houver fork (decisão de `gpui-zeron-e-fork.md`).

---

## Parte B. Mercado

### Mapa de quem faz o quê perto de nós

| Categoria | Exemplos | O que fazem | O que o xemnas tem de diferente |
| --- | --- | --- | --- |
| Memória para agentes | Mem0, Zep/Graphiti, Letta | Memória de usuário e de conversa; Zep guarda **fatos com janela de validade** (o que era verdade e quando) | Autoridade humana: nada vira decisão sem confirmação, com evidência de código |
| Conhecimento de código | Greptile, Dosu, Swimm | Documentação que se atualiza; resposta com contexto do repositório; "aprende com correções" | O *porquê* confirmado e versionado, local, sem conta |
| Registro de decisões | Log4brains, adr-tools, Backstage ADR | Arquivos Markdown, status (proposta, aceita, substituída), site estático e linha do tempo | Captura automática do que o agente decidiu e revisão em fila, em vez de escrever à mão |
| Revisão de código por IA | CodeRabbit, Greptile | Comentários por PR com "aprendizados" da equipe | Contexto é o das decisões do projeto, não o do diff |
| Triagem por teclado | Superhuman, Linear | Atalhos de uma tecla, caixa dividida, adiar, **desfazer com Z** | A Revisão já é teclado-primeiro; falta desfazer e divisão da fila |
| Grafos de conhecimento | Obsidian, Neo4j Bloom, Kumu | Camadas, zoom, agrupamento, **cenas/perspectivas salvas** | Grafo derivado de decisões, com agrupamento em escala (já feito) |
| Observabilidade de agentes | Langfuse, LangSmith | Sessões, linha do tempo, custo e latência por execução | Foco em decisões, não em chamadas; podemos mostrar custo de contexto |

Padrão que se repete: o mercado converge para **memória com tempo** (Zep),
**aprendizado a partir de correções** (CodeRabbit/Greptile) e **teclado
primeiro** (Superhuman). O espaço aberto é "memória de decisões que um humano
confirmou e que volta ao agente na hora certa", com custo baixo e local.

### Ideias de produto (da mais forte à mais exploratória)

Todas usam dados que já existem ou os de `ideias-de-produto.md`; as que pedem
backend são marcadas.

1. **Máquina do tempo do projeto.** Um controle de data no Mapa e no Contexto:
   "como era o projeto em 15 de setembro?" (decisões e regras em vigor *naquela
   data*, componentes e ligações). É o modelo bi-temporal do Zep aplicado ao
   que já guardamos (versões, `confirmed_at`, validade das claims). Nenhum
   produto de ADR faz isso de forma visual. *Backend: consulta "em vigor em D".*
2. **Guardião de mudança ("o que isto toca?").** Dado um diff, um arquivo ou um
   PR, listar as decisões e regras que o afetam, e avisar de conflito. É a
   Lente de arquivo virando comando (`xemnas check`), ferramenta MCP e um
   resumo antes de abrir PR. Inverte o CodeRabbit: em vez de opinar, lembra o que
   *foi decidido*. *Backend leve; a Lente já existe.*
3. **Pergunte ao projeto.** Caixa na paleta ("por que usamos SQLite?") que
   responde **só com decisões e regras citadas** (`D:xxxx`), primeiro por busca
   local sem modelo (custo zero) e, se a pessoa pedir, com um resumo curto. É
   o mascote com função real e fecha o ciclo "memória que ninguém lê".
4. **Revisão com desfazer e fila dividida.** `Z` desfaz confirmar, rejeitar e
   adiar por 8 s; fila dividida por origem (sessão do agente) e por confiança;
   adiar com data ("volte na segunda"). Reduz a fadiga de fila apontada em
   `ideias-de-produto.md`.
5. **Saúde da memória.** Painel que mostra decisões envelhecidas ("reconsiderar
   quando" vencido), decisões que o agente **nunca citou**, componentes sem
   nenhuma decisão mas com muita atividade (zonas cegas) e regras órfãs. Mede
   se a memória paga o custo, que é o pilar.
6. **Sala de entrada do dia ("desde sua última visita").** Tela inicial com o
   que mudou desde a última abertura: capturas novas, decisões substituídas,
   conflitos, o que o agente leu. Um resumo semanal exportável em Markdown
   sai do mesmo cálculo.
7. **Perspectivas salvas do grafo.** Camadas, câmera e foco salvos com nome
   ("Armazenamento", "Segurança"), como as cenas do Neo4j Bloom; abrir uma é
   um atalho na paleta. Leva o grafo de "bonito" para "ferramenta".
8. **Roteiro de onboarding gerado.** Para quem chega, as cinco a oito decisões
   que mais moldam o código, em ordem, com a evidência (como o Swimm, mas a
   partir de decisões confirmadas). Exporta como página.
9. **Biblioteca pessoal de padrões.** Promover uma decisão a "padrão da casa" e
   propô-la como candidata em outro projeto (que a pessoa confirma). Faz a
   memória valer entre projetos sem vazar nada sem aceite.
10. **Placar por agente.** Quem propôs o quê, taxa de aceite por agente e modelo,
    custo em tokens por projeto; ajusta o filtro de relevância e mostra qual
    agente aprende. Ecoa a observabilidade (Langfuse), com foco no que importa.
11. **Diferença entre versões de uma decisão.** Lado a lado, com a justificativa
    destacada, na linha do tempo da decisão e na Revisão de uma revisão.
12. **Publicar como site estático.** Exportar decisões e mapa como um site
    de leitura (no estilo do Log4brains), para quem não tem o app (cliente,
    novo membro). Respeita a regra de nunca escrever no repositório sozinho:
    a pessoa escolhe o destino.
13. **Voz do mascote com cuidado.** Dicas curtas e contextuais ("esta decisão
    toca 3 componentes; revisar?") apenas quando há dado real; nunca conversa
    simulada. Combina com o medidor de contexto (V5).

### Ordem sugerida

1. V1, V3, V4, V2 (lote visual de baixo custo).
2. Ideia 4 (desfazer e fila dividida) com V9: ataca o maior risco do produto.
3. Ideia 2 (guardião de mudança) e 3 (pergunte ao projeto): valor imediato
   para quem usa agentes.
4. Ideia 1 (máquina do tempo) e 7 (perspectivas): o que torna o Mapa
   inconfundível.
5. Ideias 5, 6, 10 (saúde, entrada do dia, placar) quando houver uso real.
6. Ideias 8, 9, 12 depois do dogfood.

### Em aberto

- Confirmar nas fontes originais os itens marcados "por resumo" antes de
  citá-los fora deste documento.
- Capturas de tela do Zeron em uso e de Linear/Raycast para uma lista tela a
  tela de diferenças (ver `acabamento-visual-revisao.md`).
- Medir o custo de V1 (pausa em janela inativa) e V8 (laço da marca) com
  `XEMNAS_PERF` antes de adotar.
