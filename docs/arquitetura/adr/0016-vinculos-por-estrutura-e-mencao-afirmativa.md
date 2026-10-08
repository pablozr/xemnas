# ADR-0016 — Vínculos por estrutura e por menção afirmativa

**Status:** Vigente. Aceito em 08/10/2026. Estende o [ADR-0005](0005-grafo-de-entidades.md) (grafo,
a alternativa "ler a árvore do repositório", rejeitada ali, volta de forma limitada) e o
[ADR-0014](0014-regras-com-escopo-por-componente.md) (herança do escopo da regra).

## Contexto

Numa avaliação manual de 155 vínculos decisão → componente de dois projetos reais, 91 estavam
certos, 12 duvidosos e 52 errados. As causas, todas gerais:

1. a menção tomava a primeira ocorrência do nome, sem polaridade: "independente de X", "sem X" e
   "X foi descartado" ligavam a X, e o nome dentro de outro caminho (`data_dir()/app/`) também;
2. o componente com o nome do projeto (o app `acme` do projeto `acme`) era ligado por toda frase
   sobre o produto;
3. caminhos que um ADR cita (relativos a um crate, com `../`) iam para todas as decisões tiradas
   dele e viravam proposta de componente sem existir em disco: assim nasceu o fantasma
   `examples/**`;
4. o juiz de IA dos vínculos usava o critério de todos os itens ("repete algo" descarta), e um
   vínculo sempre repete a sua decisão;
5. as regras derivadas herdavam também vínculos pendentes e eram gravadas como arestas `human`;
6. nenhum sinal estrutural era usado: quem declara a dependência, onde um símbolo é definido;
7. a raiz do workspace e a CI não eram componentes;
8. uma aresta invalidada pela máquina impedia religar para sempre.

## Decisão

1. **Menção afirmativa.** Só conta a ocorrência que não está no escopo de uma negação. A regra é
   a parte lexical do NegEx/ConText: gatilhos antes (`independent of`, `sem`, `em vez de`,
   `nem`...) e depois (`foi descartado`...), pseudo-gatilhos (`not only`, `não só`), escopo até
   pontuação de oração, palavra de terminação, conector que abre oração nova ou oito palavras.
   O `no` do inglês não é gatilho (em português é "em o"). Limite conhecido: uma negação de
   verbo ("sem perder nenhum evento no X") também alcança o substantivo. Nome que é segmento de
   outro caminho não é menção.
2. **Homônimo.** O nome do projeto (pasta, `[package].name` e `name` da raiz) restringe os
   termos de um componente que o tem: só vale como caminho, entre crases ou entre aspas. O pedido
   de vínculo à IA marca o componente "same name as the project".
3. **Índice limitado de arquivos** (relaxa o ADR-0005). Uma listagem de nomes, não de conteúdo,
   por `git ls-files` ou caminhada que pula `target`, `node_modules` e pastas ocultas (menos as
   de CI), em cache de cinco minutos por pasta, no máximo 100.000 arquivos (acima disso a
   listagem é parcial e nada é verificado), lida sob demanda e nunca na thread da interface. Um
   arquivo só conta se existe; se a decisão veio de documento, só se o texto dela o cita e não
   para excluí-lo. Proposta de componente só nasce de arquivo resolvido.
4. **Dono por manifest.** A dependência citada liga ao componente cujo manifest a declara (no
   máximo dois donos; dependência que é membro do workspace ou nome de componente fica fora). Nomes
   curtos ou comuns só valem em código.
5. **Símbolos.** Um varredor de linhas de definição (estilo ctags, sem parser, sem dependência
   nova) indexa as definições dos arquivos de código (até 256 KB cada, 32 MB no total). O símbolo
   ou o nome de arquivo entre crases liga ao componente que o define, se for um só.
6. **Raiz e CI** são componentes (`workspace` com `*` e `.cargo/**`, e `CI` por sistema), criados
   com o mapa vazio e propostos nos existentes. Não há variante nova em `WorkspaceKind`.
7. **Atores.** Cada aresta registra quem a confirmou e quem a invalidou (`person`, `rules`, `ai`,
   `inherited`; migração 0047, com preenchimento a partir do registro da revisão automática). O
   que a revisão automática adota é aresta `derived`, não `human`.
8. **Religar.** Uma aresta invalidada pela máquina deixa de bloquear quando surge um motivo
   estrutural (arquivo, dependência, símbolo) diferente do que a derrubou; a removida por pessoa
   nunca volta.
9. **Herança.** A regra herda só vínculos confirmados da decisão; se o texto dela nomeia alguns
   dos componentes, fica com eles. Invalidar o vínculo da decisão derruba o herdado, não o que uma
   pessoa confirmou na regra.
10. **Juiz de vínculos.** Lote à parte, critério próprio ("este componente é onde a regra vale?"),
    com escolha, descrição e caminhos do componente e o tipo de evidência. Os motivos estruturais
    são aceitos pelas regras, sem IA.
11. **Revalidação** a cada refresh, sem IA: o que a máquina ligou e o texto atual não sustenta é
    invalidado (nunca o de pessoa, nunca o proposto pela IA), e os componentes cuja pasta sumiu
    são listados, sem aposentar nada.

## Consequências

- Menos vínculos errados e menos chamadas de IA: os estruturais entram sem juiz e tiram
  decisões da fila de `suggest_links`. O custo novo é uma chamada a mais por passada que tem
  vínculos e outros itens.
- A listagem de arquivos e a varredura de símbolos custam CPU e memória só quando há arquivo ou
  símbolo a resolver; medidos em `link_structure_scales` (5.000 arquivos: listagem 0,07 s;
  2.000 arquivos de código: 0,5 s) e limitados por teto.
- A heurística de definição não é um parser; símbolo ambíguo não liga. A negação por léxico erra
  em frases raras; o corpus mede isso e não se ajusta olhando os dados do usuário.
- Mensagens do backend (`citado no texto`, `dependência citada`, `símbolo citado`) seguem em
  português até o ticket 21.

## Medições

Portão `link_structure_corpus` (51 decisões, 4 projetos): precisão 0,487 → 1,000 e cobertura
0,613 → 1,000; vínculos a parte negada 11 → 0; ao homônimo 4 → 0; propostas fantasma 3 → 0.
Menção: precisão 0,90 → 0,956 com cobertura 1,0. Detalhes por etapa em
[qualidade-do-nucleo.md](../qualidade-do-nucleo.md).
