# ADR-0011 — Observações descritivas automáticas de manifests

**Status:** Vigente com ajustes: o roteamento por IA citado como etapa futura já existe como opção limitada. Resultados e limites em [operação](../../operacao/avaliacoes.md).

Declarações verificáveis de Cargo.toml/package.json são observações locais, não
claims normativas, decisões confirmadas ou arestas humanas. Mantêm identidade,
versão, fonte/hash, campo, categoria, escopo, política do parser e suportes herdados.
“Declara uma dependência” não significa instalada, usada em produção ou obrigatória.
Isso evita uma fila de confirmação para fatos verificáveis sem promover inferências
a regras. A política normativa vigente não muda.

Refresh determinístico e limitado é agendado no registro/captura, startup e
reconciliação periódica. Worker local possui registry independente do worker de IA;
recovery global ocorre antes de ambos. Cache semântico persistido permite reuso
após restart. Geração/CAS impede atualização atrasada. Fonte removida/declaração
retirada invalida; falha/quota mantém histórico inelegível, sem alegar ausência.
Membership encerrado é OutOfScope, não falha transitória. Suportes de workspace
participam da igualdade/versionamento e da elegibilidade.

Context Pack/MCP/injeção/exportação compartilham a seleção descritiva por nomes
explícitos e escopo dos arquivos. Normas mantêm prioridade de orçamento. Observações
levam referências O e autoridade descritiva; correções distinguem invalidação,
incerteza e revalidação, com auditoria separada por Project/sessão/modo. A gravação
reconfere fonte, suportes, versão e dirty generation. Consulta histórica não entrega
estado atual como passado.

Este slice não chama IA para roteamento. IA para ambiguidade continua etapa futura,
com shortlist limitada, consentimento, cache e pós-validação determinística; não
será uma dependência obrigatória em cada consulta.

Quotas atuais: 64 fontes, 256 KiB/fonte, 2 MiB/refresh e 1.024 observações.
Windows valida o caminho registrado antes de resolver junctions e mantém handles
de ancestrais durante a leitura. Não há promessa de cobertura adversarial completa
de corridas, nem parser universal de código/configuração.

## Estado hoje

O roteador opcional vive em `application::context_routing`: shortlist de até 6 candidatos, no máximo 8 chamadas lógicas por projeto por dia, cache com validade e nunca obrigatório em cada consulta. As observações seguem descritivas; não viram normas.
