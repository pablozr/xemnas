# Revisão — refinamento de produto com Quiet Glass

Pesquisa em 29/09/2026. Escopo: melhorar a página existente, sem criar uma nova identidade nem prometer operações ausentes do backend. Este arquivo é local; `/docs/` permanece ignorada e fora dos commits.

## Direção recomendada

O próximo ganho de sofisticação vem de uma hierarquia mais precisa, densidade previsível e respostas claras às ações. A composição atual já tem a estrutura certa: projeto persistente, fila à esquerda, leitura à direita e ações estáveis no rodapé. Preservar essa estrutura e os detalhes explicitamente pedidos: indicador lateral, separadores, badges com bolinha e abas de arquivo coladas ao código.

Minha proposta é **“leitura editorial, ferramentas discretas”**: título forte e conteúdo confortável; metadados menores; superfície de evidência com ergonomia de editor; ações e feedback com aparência e comportamento consistentes. Não acrescentar efeitos luminosos, novas cores, gráficos de confiança ou cartões repetidos.

## O que foi observado no Xemnas

Fontes locais consultadas: `VISUAL-IDENTITY.md`, `apps/desktop-gpui/src/screens/inbox.rs`, `evidence.rs`, `review_editor.rs` e captura `docs/design-review/workspace-polished.png`. As observações abaixo são do produto atual, não afirmações das referências externas.

- A captura confirma seleção persistente na lateral, dot de estado, contagem na aba, fontes em painel contínuo e ações fora da rolagem. Isso deve permanecer.
- A lista usa texto integral, sem limite de linhas para pergunta e escolha. Um candidato longo pode ocupar boa parte da coluna de 320 px e prejudicar a comparação com os demais.
- Todas as datas da demonstração repetem o mesmo dia. Estado e data têm lugar definido, mas o tratamento de metadados ocupa uma proporção considerável de cada linha.
- O detalhe oferece divisores claros e um título forte. Escolha, motivo, confiança e origem recebem atualmente um tratamento de seção muito semelhante, embora sua relevância na revisão seja diferente.
- As fontes já são cacheadas e renderizadas por linhas virtualizadas. É uma base real de escala, não uma necessidade a reimplementar. Faltam, no entanto, meios explícitos para selecionar/copiar trechos no painel mostrado; o código é composto por elementos de texto de leitura.
- A rolagem da fonte intercepta a roda do mouse para não mover o detalhe. É preciso verificar ergonomia nos extremos e com fonte curta, para não transmitir a sensação de a página ter travado.
- Cada aba de evidência é um botão focalizável e aceita Enter/Espaço. Falta o comportamento de um conjunto de abas: setas, foco agrupado e associação acessível com o painel.
- Os botões bloqueiam operações duplicadas, mas o texto não identifica qual operação está sendo processada; cursor e aparência ainda são muito próximos do estado disponível.
- Avisos de sucesso e erro são acrescentados antes do corpo da revisão. Essa faixa altera a posição vertical do conteúdo e pode reduzir a continuidade da leitura.
- Ajustar reutiliza três campos de uma linha, inclusive para o motivo. O código preserva quebras de linha originais quando não houve edição, mas editar um texto longo continua desconfortável. Os botões do formulário ficam dentro da área rolável.
- A linguagem alterna “Revisão” e “Inbox” em mensagens de erro. Isso expõe vocabulário interno desnecessário.

## Referências primárias e o que aproveitar

| Referência | Observação da fonte | Interpretação para o Xemnas |
| --- | --- | --- |
| [Linear — A calmer interface for a product in motion, 12/03/2026](https://linear.app/now/behind-the-latest-design-refresh) | O refresh reduz o peso da navegação, compacta abas, usa menos ícones e suaviza divisões. Ações previsíveis e foco no conteúdo orientam a mudança. | Conservar os divisores pedidos, tornando os secundários mais discretos. Dar mais peso à decisão em análise que à navegação. Usar ícones onde ajudam a identificar uma ferramenta ou fonte, sem colocar um em cada título. |
| [Linear — How we redesigned the Linear UI, 28/03/2024](https://linear.app/now/how-we-redesigned-the-linear-ui) | O projeto trabalhou alinhamento vertical e horizontal de rótulos, ícones e botões, além de hierarquia e densidade. | Revisar baselines, alturas e margens em uma grade comum. Não copiar cores nem reproduzir um issue tracker. |
| [Zed — Appearance](https://zed.dev/docs/appearance), [All Settings](https://zed.dev/docs/reference/all-settings) | A documentação separa tipografia por contexto e oferece configuração da scrollbar e de seus indicadores. Estas páginas foram localizadas pela busca oficial; a abertura integral falhou no ambiente. | Manter texto de interface e fonte monoespaçada separados. O princípio aproveitável é a clareza da leitura de código; não introduzir minimap ou marcadores de diagnóstico que nossos artefatos não têm. |
| [Raycast — Keyboard Shortcuts](https://manual.raycast.com/keyboard-shortcuts) | Listas têm navegação por setas; ações possuem atalhos e o painel contextual permite descobri-las. | Facilitar revisão sequencial e mostrar atalhos perto da ação. Preservar Ctrl K como busca já existente no Xemnas, mesmo que Raycast o use para outra finalidade. |
| [Microsoft — Progress controls](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/progress-controls) | Diferencia progresso determinado e indeterminado e recomenda feedback adequado ao alcance do bloqueio. Para coleções, recomenda indicação conjunta, em vez de uma por item. | Mostrar “Confirmando…” no botão acionado e uma indicação discreta de atualização da fila. Nunca inventar percentual ou tempo restante. |
| [Microsoft — Scroll viewer controls](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/scroll-controls) | Recomenda rolagem vertical para textos longos e dois eixos quando o conteúdo exige ambos. Scrollbars não devem encobrir conteúdo interativo. | Prosa com quebra de linha; código preservado com dois eixos. A scrollbar precisa ser perceptível e alcançável sem cobrir abas, ferramentas ou texto. |
| [Microsoft — Text box](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/text-box) | Recomenda edição multilinha para textos longos, altura limitada e contador quando existe limite de caracteres. | Transformar Escolha e Motivo em campos multilinha; exibir limites reais e validação junto ao campo. Não fazer o formulário crescer indefinidamente. |
| [W3C — Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) | Descreve um grupo de abas com selecionada identificada, navegação por setas e foco ordenado. Ativação automática é adequada quando o painel está disponível sem latência. | Aplicar comportamento equivalente no GPUI/AccessKit. A recomendação é de interação e semântica; WAI-ARIA é uma especificação web, não uma API nativa a copiar literalmente. |

## Plano priorizado

### Primeiro: acabamento que muda a experiência

**1. Feedback ancorado na ação.** No rodapé atual, o botão acionado muda para “Confirmando…”, “Rejeitando…”, “Adiando…” ou “Salvando…”. Os outros ficam indisponíveis com estado acessível, além da proteção funcional já existente. Sucesso discreto com ícone de check e mensagem curta, em área estável da barra: “Decisão confirmada” ou “Candidato adiado”. Erro permanece visível com recuperação pertinente. Não remover erro automaticamente nem acrescentar “Desfazer” sem operação real que o sustente.

**2. Revisão sequencial por teclado.** Setas para mover seleção na fila quando o foco está nela, sem interceptar edição de texto; rolagem acompanha a linha selecionada. Um atalho modificado, por exemplo Ctrl Enter, pode confirmar quando o foco pertence à superfície de revisão e não existe formulário aberto ou operação em curso. Mostrar esse atalho no botão ou tooltip. Enter isolado continua ativando o controle focado, para evitar confirmações acidentais. Após remover um candidato da fila, mover foco para o próximo candidato visível, em vez de mandá-lo ao botão Atualizar.

**3. Ajustar como formulário de produto.** Pergunta com duas linhas visíveis; Escolha com três; Motivo com cinco, alturas limitadas e rolagem interna quando necessário. Limites reais de 500/1.000/4.000 caracteres, erros por campo e ação salvar explícita. A barra “Cancelar / Salvar ajustes / Salvar e confirmar” deve permanecer fora da rolagem, como a barra da leitura. Nenhum texto de campo vira placeholder no lugar de um rótulo.

### Depois: polimento visual de maior retorno

**4. Linhas da fila com densidade previsível.** Manter coluna 320 px. Padding lateral 16 px; topo/rodapé 12–16 px; título até duas linhas; resumo até duas linhas; data e badge na mesma linha. Altura típica aproximada de 120–136 px, sem impor corte do texto completo no detalhe. Tooltip ou detalhe acessível oferece o conteúdo integral truncado. O fundo selecionado, indicador lateral de 2 px e foco continuam distintos e persistem sob hover.

**5. Hierarquia editorial no detalhe.** Preservar título 24 px e padding de 32 px. Escolha deve ser o primeiro bloco legível; Motivo permanece prosa secundária. Evidências continuam imediatamente abaixo. Confiança e Origem podem formar um único grupo “Contexto da captura” no fim, com rótulos menores e os valores reais; sem duplicar informação no cabeçalho. Em janela larga, limitar linhas de prosa a cerca de 75–85 caracteres, enquanto a evidência usa a largura disponível. É uma medida proposta, a validar na janela nativa, não um número exigido pelas referências.

**6. Anatomia da evidência.** Preservar uma borda externa e a faixa de abas anexa. Abaixo dela, toolbar compacta: caminho registrado à esquerda, ação “Copiar trecho” à direita e “Expandir leitura” se implementada como estado local reversível. Mostrar intervalo real quando `start_line` existe; sem metadado, indicar “linhas do trecho”. Copiar somente o conteúdo redigido recebido do backend. Seleção de texto, cópia de seleção e feedback “Copiado” são mais úteis que adicionar ícones decorativos ao código. Prosa em `user_text` pode ter quebra de linha; diffs/código mantêm whitespace e rolagem horizontal. Não chamar tudo de código nem inferir arquivo, linguagem ou origem ausentes.

**7. Abas e scroll com sinais discretos.** Abas de fonte até 200 px, filename truncado com caminho completo acessível; selected mantém lavender mineral. Foco permanece independente da seleção. Setas navegam e trazem a aba ativa à área visível. Em fontes longas, scrollbar visível ao interagir e indicação textual da extensão, como “1.500 linhas”, deixam a escala compreensível. Evitar fades que escondem conteúdo ou sugerem overflow quando ele não existe. Verificar roda, trackpad e teclado, inclusive ao chegar aos extremos.

### Refinamento final de consistência

- Alinhar as três colunas e o rodapé em incrementos existentes de 4/8 px. Unificar dimensões de ícones, radius, hover, foco e área clicável; não espalhar novos literais de cor.
- Usar “Revisão” nas mensagens ao usuário; “Inbox” fica no código. Singular correto: “1 fonte”, “1 candidato carregado”.
- Em filtro local, manter a contagem total na aba e tornar o alcance explícito: “3 de 50 carregados”. Isso não significa pesquisar toda a base. Sem filtro, o contador pode ser visualmente mais discreto, preservando a informação prevista pela identidade.
- Diferenciar fila vazia, filtro sem resultado e falha. Filtro sem resultado oferece limpar busca; falha oferece atualizar; fila vazia orienta sobre captura/extração, sem botão que aponta para uma tela inexistente.
- Microtransições, se usadas, devem comunicar mudança de hover/foco sem atrasar leitura ou ação. Respeitar preferência de movimento reduzido. Não aplicar shimmer contínuo ou animação de entrada a cada candidato.

## Proposta de composição

```text
Projeto                              [Revisão 5] [Detalhes]
──────────────────────────────────────────────────────────
Fila 320 px                         Leitura
 Aguardando revisão  Atualizar       ● Pendente   29 set
 [Filtrar carregados       Ctrl K]   Pergunta do candidato
 5 carregados                       ────────────────────
                                    Escolha sugerida
 29 set                 ● Pendente  Texto de decisão
 Pergunta, até duas linhas           ────────────────────
 Escolha, até duas linhas            Motivo
                                    Prosa de leitura
 ...                                ────────────────────
                                    Evidências · 2 fontes
                                    ┌[file inbox.rs][file captura.txt]┐
                                    │ src/inbox.rs:24–27      Copiar │
                                    │ 24  transaction.begin();      │
                                    │ ...                           │
                                    └───────────────────────────────┘
                                    Contexto da captura
                                    Confiança / origem reais
                                    ────────────────────
                                    Feedback  Rejeitar Adiar Ajustar
                                                 Confirmar Ctrl ↵
```

O diagrama é especificação de estudo. Não representa funcionalidade já implementada e não altera a ordem das abas existentes. “Copiar”, campos multilinha e novo atalho exigem implementação de interface, mas não novos casos de uso de domínio.

## Critérios para considerar o refinamento pronto

1. Conferir tamanho padrão e compacto, escala 100% e 125%, nomes/caminhos longos, títulos com 500 caracteres e motivo extenso.
2. Rever 50 candidatos carregados e 1.500 linhas de fonte; preservar virtualização atual e não duplicar strings por frame. Se múltiplas páginas tornarem a lista lenta, medir antes de migrar a fila para virtualização com altura previsível.
3. Navegar por teclado entre fila, abas e ações. Acessibilidade anuncia seleção, status, indisponibilidade e mensagens; não depende da cor da bolinha.
4. Confirmar, rejeitar, adiar, retomar, ajustar, cancelar e falhar sem perder orientação. Contagem permanece da fila inteira; busca local não altera seu significado.
5. Verificar fontes sem caminho, sem linha original, não código, redigidas, vazias e longas. Nenhum metadado é inventado.
6. Não introduzir métricas, avatares, histórico falso, botão abrir editor, undo ou painel de diagnóstico sem dados/operação reais. Sofisticação deve resultar da precisão do que já existe.

## Limites da pesquisa

É uma análise do código, identidade e screenshot atual, combinada com referências oficiais. Não foi realizado um teste adicional da janela nativa nesta pesquisa, nem testada a disponibilidade de APIs GPUI para cada interação proposta. As medidas e prioridades são recomendações de design para implementação e validação posterior. As referências de Zed ficaram restritas ao conteúdo indexado oficial porque a leitura integral falhou; as demais recomendações centrais têm fontes abertas diretamente ou conteúdo primário devolvido pela busca.
