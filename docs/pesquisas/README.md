# Pesquisas

Pesquisas de referência, investigações técnicas e ideias para o futuro. Nada aqui
é escopo aprovado: escopo está em `docs/produto/MVP-SPEC.md`, nos ADRs e nos
planos de `docs/roadmap/`.

## Índice

| Documento | Assunto | Status |
| --- | --- | --- |
| [ideias-de-produto.md](ideias-de-produto.md) | Revisão, contexto do projeto e próximas ideias do produto | Aberta. A aba Contexto já foi entregue (`40ead8a`); o resto segue em discussão até o dogfood |
| [provedores-e-modelos-de-ia.md](provedores-e-modelos-de-ia.md) | Provedor e modelo para a extração (conta ChatGPT, OpenCode, modelo local, chave de API) | Aberta. Não implementada; endpoints marcados "(confirmar)" precisam de conferência |
| [glass-na-janela-inteira.md](glass-na-janela-inteira.md) | Material do Windows (Mica, Acrylic) na janela e conteúdo em card | Base de decisão. O glass é opcional (`XEMNAS_BACKDROP`); a regra vigente está em `docs/design/VISUAL-IDENTITY.md` |

## Como usar esta pasta

- **Uma pesquisa por assunto**, com nome descritivo em minúsculas e hífens
  (`provedores-e-modelos-de-ia.md`, não `research-3.md`).
- **Cabeçalho obrigatório:** título, data da pesquisa, a pergunta que ela responde
  e uma linha `**Status:**` com um destes valores:
  - **Aberta**: ideia ou investigação para o futuro; ainda não virou trabalho.
  - **Em implementação**: virou tela ou recurso em andamento; o plano pode viver
    aqui enquanto isso.
  - **Base de decisão**: já implementada e citada por uma regra vigente como
    fundamento.
- **Fontes com link.** O que veio de resumo de busca, ou não pôde ser conferido,
  fica marcado como "(confirmar)".
- **Ao implementar:** leve o que for duradouro para o documento oficial
  (`docs/design/VISUAL-IDENTITY.md`, o design system, `docs/arquitetura/` ou um ADR)
  e apague a pesquisa e as imagens ou protótipos que só ela usava, no mesmo commit.
  Só fica quem uma regra vigente cita como fundamento, com o status
  **Base de decisão**.
- **Atualize este índice** sempre que entrar, mudar de status ou sair uma pesquisa.
