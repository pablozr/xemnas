# Pesquisas

Pesquisas de referência, investigações técnicas e ideias para o futuro. Nada aqui
é escopo aprovado: escopo está nos ADRs, em `docs/produto/` e nos planos de
`docs/roadmap/`.

## Índice

| Documento | Assunto | Status |
| --- | --- | --- |
| [backlog-de-ideias.md](backlog-de-ideias.md) | O que ainda está aberto e concreto, por tema, com a origem de cada linha | Aberta |
| [mercado-e-ideias.md](mercado-e-ideias.md) | Concorrentes e onde colidem, ideias com evidência mais forte, estudos de dores e o que foi descartado | Aberta |
| [ai-memory-do-akita.md](ai-memory-do-akita.md) | O ai-memory do Akita: comparação, cinco ideias sem IA nova para trazer e por que não integrar agora | Aberta. Estudo de referência; nada implementado |
| [sonho-consolidacao-da-memoria.md](sonho-consolidacao-da-memoria.md) | "Sonho": rodada periódica que junta duplicatas, resolve contradições, aposenta o envelhecido e descobre ligações; exportação no formato Agent Memory Repo | Aberta. Desenho e protocolo; nada implementado |
| [implementacao-procurador-e-mapa.md](implementacao-procurador-e-mapa.md) | Onde o Procurador e o mapa de cegueira entram no código: sinais de turno, autonomia, livro em sombra, contratos e ordem de entrega | Aberta. Estudo de implementação; nada implementado |
| [procurador.md](procurador.md) | Procurador: o Xemnas responde ao agente pela pessoa quando uma decisão confirmada cobre a pergunta | Aberta. Desenho e protocolo; nada implementado |
| [mapa-de-cegueira.md](mapa-de-cegueira.md) | Código escrito por agentes sem conversa com a pessoa, por componente, priorizado por risco, com tour de compreensão | Aberta. Desenho e protocolo; nada implementado |
| [ideias-de-virada.md](ideias-de-virada.md) | Decisões como diagnósticos (LSP), repetições que viram regra, barramento entre agentes paralelos e contrato do repositório | Aberta. Pesquisa e proposta; nada implementado nem medido |
| [precisao-do-contexto.md](precisao-do-contexto.md) | Precisão do bloco entregue ao agente: evidência, diagnóstico, técnicas por custo, medição e plano | Em implementação. Menções, termos de busca e regras com escopo entregues; v4 às cegas 0,58 / 0,62 |
| [busca-semantica-local.md](busca-semantica-local.md) | Embeddings e busca vetorial locais: reprovados no contexto (v4 e v5); biblioteca de PDFs não iniciada | Aberta |
| [ciclo-correcao-verificacao-enforcement.md](ciclo-correcao-verificacao-enforcement.md) | Aprender com as correções do usuário, conferir o diff contra as regras e promover regras violadas a testes | Aberta. Ideias anotadas; nada implementado |
| [glass-na-janela-inteira.md](glass-na-janela-inteira.md) | Material do Windows (Mica, Acrylic) na janela e conteúdo em card | Base de decisão. A regra vigente está em `docs/design/VISUAL-IDENTITY.md` |

## Como usar esta pasta

- **Uma pesquisa por assunto**, com nome descritivo em minúsculas e hífens.
- **Cabeçalho obrigatório:** título, data, a pergunta que ela responde e uma linha
  `**Status:**`: **Aberta** (ainda não virou trabalho), **Em implementação** (o plano
  vive aqui enquanto isso) ou **Base de decisão** (já implementada e citada por uma
  regra vigente como fundamento).
- **Fontes com link.** O que veio de resumo de busca, ou não pôde ser conferido,
  fica marcado como "(confirmar)".
- **Ao implementar,** leve o que for duradouro para o documento oficial
  (`docs/design/VISUAL-IDENTITY.md`, `docs/arquitetura/` ou um ADR), deixe o que sobrou
  de aberto numa linha do `backlog-de-ideias.md` e apague a pesquisa e as imagens que só
  ela usava, no mesmo commit. O git guarda o histórico. Só fica quem uma regra vigente
  cita como fundamento.
- **Atualize este índice** quando entrar, mudar de status ou sair uma pesquisa.
