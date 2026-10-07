# 26 — Fluxos didáticos na Visão

**What to build:** Na página HTML da Visão (`crates/application/src/page/template.html`, montada por `page/mod.rs`), explicar cada fluxo técnico do projeto de forma didática, sob demanda e com fonte citada. O protótipo feito à mão para o fluxo "como o contexto chega ao agente" foi aprovado pelo usuário em 07/10/2026 (artifact privado: https://claude.ai/artifact/1jRhKBVoXpEEKffR7YHuMW).

**Blocked by:** dogfood (ticket 20).

**Status: open**

- [ ] Diagrama C4 em três níveis (contexto, contêineres, componentes) derivado do grafo, sem IA, com traço de quadro branco (rough.js ou SVG equivalente embutido, já que a página é offline).
- [ ] Layout do diagrama sem linhas cruzadas e com todos os rótulos legíveis. No protótipo, as posições foram dadas à mão e as setas cruzam caixas e textos no nível de componentes. Usar um layout em camadas (por exemplo, o algoritmo do dagre ou do ELK) ou rotear as arestas ortogonalmente, com rótulos fora das caixas.
- [ ] Passos em modo história: o passo aberto destaca no diagrama os componentes que ele usa.
- [ ] Para cada passo: o problema em uma frase, onde fica no código, o exemplo concreto do próprio projeto, o código real das capturas ou um trecho marcado como "simplificado", e as fontes.
- [ ] "Por que é assim" e "o que não resolve": cada frase cita uma decisão ou um documento, com validação textual da citação, como na extração.
- [ ] Dois níveis de leitura, "Explicar fácil" e "Explicar técnico", e glossário ao passar o mouse, vindo dos termos do projeto.
- [ ] Selo "desatualizada" quando o fingerprint das fontes mudar.

**Custo:** uma chamada de IA por fluxo, só quando o usuário pedir, com cache pelo fingerprint das fontes. O C4 e o layout não usam IA. Antes de implementar, estimar o tamanho do prompt e o número de fluxos por projeto, e combinar o teto com o usuário.

**Aceite:** o fluxo "como o contexto chega ao agente" gerado a partir do Xemnas fica no mesmo nível do protótipo, sem linhas cruzadas, nas duas paletas e numa janela estreita, e nenhuma frase fica sem fonte.
