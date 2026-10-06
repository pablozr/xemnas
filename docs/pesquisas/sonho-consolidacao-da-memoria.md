# Sonho: consolidação periódica da memória

**Data:** 06/10/2026.
**Pergunta:** uma rodada periódica, em segundo plano, que limpa e enriquece a memória do
projeto (junta duplicatas, resolve contradições, aposenta o que envelheceu, descobre ligações
e padrões) deixa o contexto entregue ao agente mais certeiro, sem custo que fira o pilar?
**Status:** Aberta. Desenho e protocolo; nada implementado.

## De onde vem a ideia

Em 05/10/2026 a Cognition anunciou o *Dreaming* no Devin e um padrão aberto, o
[Agent Memory Repo](https://cognition.com/agent-memory-repo)
([anúncio](https://x.com/cognition/status/2107165034463867001)):

- **Memória como repositório git**: um `MEMORY.md` curto que o agente lê em toda sessão,
  notas em Markdown com uma entrada por linha e metadados (`[source: …; added: …]`), links
  `[[caminho]]` entre notas, como uma wiki. A cada sessão: clonar, buscar, atualizar, *push*.
- **O agente grava sozinho**, sem pessoa no caminho.
- **Dreaming**: um agente dedicado que roda de tempos em tempos com dois trabalhos:
  acrescentar (padrões entre sessões viram entradas novas) e limpar (juntar duplicatas,
  remover o que envelheceu, conferir as fontes para resolver contradições).
- **Composição**: memórias de várias pessoas carregadas na mesma sessão, cada uma no seu
  repositório, com dono e histórico próprios.

## O que o Xemnas já tem e o que falta

| Peça do Dreaming | No Xemnas hoje |
| --- | --- |
| Fonte de cada fato | toda decisão e regra cita a captura, o artefato e a versão de origem |
| Contradição | relação `conflita com` e substituição explícita; nada procura conflitos sozinho |
| Envelhecimento | validade (`valid_until`), substituição; nada percebe uma decisão que cita um arquivo apagado ou que ninguém usa |
| Duplicata | a triagem descarta o candidato que repete uma pergunta já registrada; decisões já adotadas não são comparadas entre si |
| Ligações | arquivo, dependência, menção e, desde 06/10, ligação proposta pela IA na adoção e na atualização do mapa |
| Padrões entre sessões | regras derivadas de uma decisão (`claim_suggestions`); nada olha várias decisões juntas |
| Juiz | revisão automática com IA em lote, ledger e desfazer |

A diferença de fundo continua: no Xemnas a memória é **revisada e ligada ao código**, não
escrita livremente pelo agente. O Sonho não muda isso; ele **propõe**, e a revisão (manual ou
automática) decide.

## Desenho

Um job `dream` por projeto, na raia de sugestões, quando o app está ocioso e no máximo uma vez
por dia (ou sob demanda). Cada passo produz **propostas**, nunca apaga nada:

1. **Duplicatas.** Pares de decisões em vigor com pergunta e escolha equivalentes. Candidatos
   baratos pelo léxico (a ponte PT/EN e os termos de busca já existem); só os pares candidatos
   vão ao modelo. Proposta: substituir a mais antiga pela mais completa, com o motivo.
2. **Contradições.** Pares do mesmo componente (o grafo restringe a busca) cujas escolhas se
   excluem. Proposta: relação `conflita com`, ou substituição quando a data e a fonte
   mostram qual vale.
3. **Envelhecimento.** Sinais locais, sem IA: arquivos citados que não existem mais, componente
   aposentado, decisão nunca entregue ao agente em N semanas (as entregas já são medidas).
   Proposta: revisar ou encerrar a validade, com o sinal que a motivou.
4. **Ligações que faltam.** Decisões em vigor sem ligação ao mapa passam pela ligação da IA
   (`suggest_links`); componentes novos do mapa reabrem as decisões antigas.
5. **Padrões.** Três ou mais decisões que repetem a mesma restrição viram uma **regra**
   proposta (claim), com as decisões como fonte. É o "acrescentar" do Dreaming.

O Sonho grava um relatório curto (o que encontrou e propôs) que a Revisão mostra numa linha,
como o "Feito sozinho".

## Como medir sem se enganar

O mesmo protocolo da seleção de contexto e da ligação pela IA:

- **Corpus de memória sintética, escrito às cegas** por um agente que não vê o código: um
  projeto com ~40 decisões e ~6 componentes, com duplicatas, contradições, decisões velhas e
  padrões plantados e rotulados, e negativas de mesmo vocabulário (decisões parecidas que não
  se repetem, escolhas diferentes que não se contradizem).
- **Respostas do modelo como fixture**, geradas uma vez com autorização do usuário; o portão
  mede precisão e cobertura de cada passo.
- **Holdout selado**: um segundo corpus, escrito às cegas, só lido no fim.
- **Efeito no que importa**: rodar o corpus de contexto (v3/v4/v5) antes e depois do Sonho;
  ele só fica se a precisão do contexto não cair.
- **Custo**: chamadas por rodada, tempo e CPU, com teto antes de começar.

## Interoperabilidade: Agent Memory Repo

Exportar o que está confirmado no formato aberto: um `MEMORY.md` com o essencial e o índice,
uma nota por componente com as decisões e regras que se aplicam a ele, cada entrada com
`[source: …; added: …]` e `[[links]]` entre componentes. Qualquer agente que fale o padrão
passa a usar a memória do Xemnas. A importação (memória de outro agente virando candidatos à
revisão) vem depois, se a exportação tiver uso.

## Outras ideias para explorar

- **Memória que esquece pelo uso**: o que nunca é entregue nem aberto por `get_decision`
  perde peso na seleção; o que o agente consulta ganha.
- **Sinal do agente**: quando o agente abre uma decisão pelo MCP, aquilo foi útil; quando
  contraria uma regra entregue, a regra ou a entrega está errada.
- **Memória ciente de branch**: decisões de worktree valem só na branch até o merge;
  o Sonho promove ou expira ([ticket 25](../roadmap/tickets/25-memoria-ciente-de-branch.md)).
- **Ensaio contrafactual**: repetir uma tarefa real com e sem o bloco injetado e comparar o
  resultado, para medir o valor do contexto em vez de só a precisão.

## Plano proposto

1. Corpus às cegas e holdout (sem código novo).
2. Passos 3 e 4 primeiro (sinais locais e ligação que já existe): custo quase zero.
3. Passos 1 e 2 com fixture e portão.
4. Passo 5 por último.
5. Exportação no formato Agent Memory Repo, independente dos passos acima.
