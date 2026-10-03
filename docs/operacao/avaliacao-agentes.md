# Avaliação supervisionada do Xemnas por agentes

Data: 03/10/2026. Base de produção: `37bcc2f`.
Status: piloto controlado concluído; ciclo completo de uso cotidiano ainda não validado.

O usuário pediu que um GPT-6 Luna com raciocínio médio executasse tarefas e que
o coordenador verificasse o uso e o núcleo do Xemnas. Este protocolo distingue
contratos técnicos, recuperação por um agente novo e benefício numa tarefa real.
Não substitui a semana de [dogfood](dogfood-log.md).

## Ciclo proposto

1. **Preparar:** escolher uma tarefa de desenvolvimento num projeto isolado,
   registrar antes as decisões esperadas e os erros críticos, separar os dados
   pessoais e congelar a versão do produto. Não revelar respostas nos prompts.
2. **Executar:** um Luna medium trabalha na tarefa; guardar suas consultas,
   alterações, comandos, falhas e a conversa original. O coordenador acompanha
   atritos sem ensinar respostas. Registrar qualquer intervenção.
3. **Capturar e revisar:** verificar a captura original, os candidatos e suas
   evidências. Medir propostas úteis, descartadas e corrigidas, e o esforço de
   revisão. A confirmação real continua sendo humana; adoção por script só
   pode ser um controle de laboratório, identificado como tal.
4. **Retomar com agentes novos:** iniciar sessões sem o histórico anterior em
   três condições: código sem motivos registrados, código com ADRs equivalentes
   e código com acesso ao Xemnas. Mesma tarefa, conhecimento equivalente nos dois
   últimos controles e prompts sem as respostas. Alternar a ordem das condições
   nas rodadas seguintes e variar o cenário, em vez de reutilizar uma resposta.
5. **Auditar:** conferir decisões, versões, evidências, mudanças e testes contra
   o registro anterior. Verificar se a decisão substituída entrou como vigente,
   se faltas de informação foram reconhecidas e se o agente precisou de ajuda.
6. **Dar o parecer e repetir:** classificar achados como falha de implementação,
   atrito de operação, limite conhecido ou hipótese de produto. Corrigir um
   problema por vez, quando autorizado, e repetir a tarefa que o revelou antes
   de expandir o escopo. Um resultado negativo também encerra uma rodada.

O parecer de cada rodada deve dizer o que aconteceu, o que foi conferido,
qual trabalho foi poupado ou acrescentado e se eu escolheria continuar usando.
Testes verdes sozinhos não aprovam o produto. Uma decisão crítica incorreta,
uma versão substituída tratada como vigente ou uma justificativa inventada
reprovam a fidelidade daquela tarefa. A utilidade exige comparar também o custo
de produzir e manter a memória com o de manter os ADRs.

## Piloto executado

Foram usados um executor técnico e três agentes GPT-6 Luna medium novos,
sem herdar esta conversa, para as condições de recuperação.
O cenário RelayDesk é sintético: fila desktop, uma constante SQLite no código,
uma decisão antiga de PostgreSQL, sua substituição por SQLite individual offline
e uma decisão vigente de busca lexical FTS5. Não existe regra de retenção.

O [runner](avaliacao-agentes/runner.rs) usa os casos de uso e os crates de
produção, um banco SQLite em disco e a API HTTP local real. Ingere três envelopes
diretamente pelo caso de uso, repete cada ingestão para conferir deduplicação,
executa o `FakeCandidateExtractor`, edita e confirma os candidatos por script,
registra a substituição e serve a API para o binário MCP real. Não monta a UI,
workers de extração ou o plugin OpenCode. O adapter dos envelopes está
identificado como `controlled-experiment`, não como uma captura real OpenCode.

A extração real offline gerou a pergunta genérica
“Qual decisão durável a captura registra sobre public_contract?” nas três
capturas. As escolhas semânticas finais foram fornecidas por `CandidateEdits`
no laboratório. Portanto, esse piloto não prova qualidade de extração por IA
nem redução do esforço de revisão humana.

O processo foi encerrado e o banco reaberto antes das consultas dos agentes.
Uma tentativa de reinício do runner falhou por tentar cadastrar novamente o
projeto; isso foi um erro do instrumento de avaliação, corrigido no runner,
não um defeito atribuído ao Xemnas.

### Recuperação pelos agentes

Todos receberam as mesmas cinco perguntas: persistência e motivo, serviço
separado/rede, mecanismo de busca/sinônimos, escolha substituída e retenção.

| Condição | Resultado observado | Limite da comparação |
| --- | --- | --- |
| Código sem memória | Identificou SQLite; declarou os motivos, busca, histórico e retenção ausentes | Abstinência correta; o conhecimento estava ausente por construção |
| Código + Xemnas | Recuperou os quatro assuntos registrados, citou referências/versões, distinguiu PostgreSQL substituído e não inventou retenção | Precisou de intervenção sobre transporte MCP por terminal |
| Código + ADR equivalente | Recuperou os mesmos quatro assuntos, com arquivos/linhas; não inventou retenção | O controle é um documento pequeno e fácil de ler |

O agente com Xemnas inicialmente conseguiu `initialize`/`tools/list`, mas suas
invocações interativas de `tools/call` encerravam sem saída. O coordenador
orientou o uso de JSON-RPC com `id` e pipeline PowerShell; as consultas passaram
a responder. As respostas esperadas não foram reveladas. A causa específica
da tentativa interativa não foi diagnosticada; não atribuir o incidente à busca
ou afirmar que o agente corrigiu sozinho a invocação.

O agente relatou consultas individuais de aproximadamente 22–35 ms. Não há uma
medição comparável de tempo total de tarefa, incluindo preparação e intervenção.
Esses tempos relatados não demonstram ganho de produtividade.

O agente chamou de equivalente uma consulta sobre spam e mensagens indesejadas.
O coordenador rejeitou essa classificação: o tema não corresponde às decisões
de busca do cenário. Não contar essa consulta como prova de recuperação de
sinônimos. A auditoria independente usou `localizar por significado`: não houve
resultado, embora `sinônimos` recuperasse a decisão FTS5. É um exemplo controlado
da dependência de vocabulário lexical, não uma taxa de qualidade da busca.

### Conferência independente

O coordenador consultou o MCP real novamente e confrontou as respostas com o
[oráculo sintético](avaliacao-agentes/oracle-piloto.json) e o banco em leitura.
As [requisições e respostas de auditoria](avaliacao-agentes/mcp-auditoria-piloto.json)
estão preservadas. Elas são uma repetição independente, não o log completo do
agente executor. Os comandos dos testes ficaram no histórico desta sessão;
não há stdout completo versionado da bateria.

- SQLite vigente: `D:90d418bb v1`; PostgreSQL substituído: `D:c55b943b v1`.
- Busca FTS5 vigente: `D:4f766fd8 v1`, incluindo a ausência de aprovação semântica.
- Busca por retenção retorna itens sem essa informação; o agente corretamente
  não confundiu resultado de busca com resposta à pergunta.
- `file_context(src/storage.rs)` retorna vazio: não foram cadastradas entidades
  ou vínculos no mapa. Isso mostra uma pré-condição do recurso, não um bug.
- O retorno de `get_decision` mostra o número de artefatos; nesse retorno, o
  agente não abriu os textos das fontes. Não chamar sua citação de auditoria
  independente da evidência original.

A [verificação do banco](avaliacao-agentes/verificacao-piloto.json) encontrou um
projeto, três capturas, seis artefatos, três candidatos editados e aceitos, três
decisões, três revisões, seis vínculos de evidência e uma relação. Duas decisões
estavam aceitas e uma substituída. As capturas repetidas não duplicaram esses
registros. Os hashes do cenário são um registro final; não são uma comparação
automatizada antes/depois.

### Validação técnica e limitações

- `cargo test --locked -p application -p storage-sqlite -p local-api -p mcp-server`:
  bateria interrompida pelo Windows Application Control, erro 4551, ao executar
  `application --test export`. Antes disso passaram application lib 122,
  analysis 6, decisions 6 e diagnostics 3 testes. Não declarar a bateria verde.
- `cargo test --locked -p storage-sqlite --test full_cycle`: 1/1 passou. Prova
  registro, ingestão, evidência, edição, confirmação, busca/exportação e ausência
  de mutação do projeto, com dados sintéticos e extrator determinístico.
- `cargo test --locked -p mcp-server --test stdio`: 1/2 passou. A falha em
  `apps/mcp-server/tests/stdio.rs:101` espera duas ferramentas; o servidor retorna
  três. É uma asserção desatualizada; a auditoria por MCP confirmou as três
  ferramentas operando. Esse teste usa API falsa.
- O teste individual de modo de contexto da API foi cancelado antes de executar,
  enquanto aguardava o lock do Cargo; sua cobertura foi apenas lida.
- O runner compilou e executou com sucesso; sua versão preservada passou
  `cargo check --locked --offline` e Clippy com `-D warnings` pelo manifest
  temporário. O script de preparação também foi executado e compilou o runner.
  A tentativa final de executar o runner recompilado foi bloqueada pelo
  Windows Application Control; a reexecução desse artefato ficou sem validação.
- `rustfmt` foi bloqueado pelo Windows Application Control, inclusive por caminho
  direto. As linhas longas do runner foram revisadas e quebradas manualmente;
  a formatação automática não foi validada.

Não houve acesso a banco pessoal, credenciais, provedor externo ou controle de
foco da interface. Nenhum código de produção foi alterado. O servidor temporário
foi encerrado depois da auditoria. Não houve push.

## Parecer do piloto

O núcleo de armazenamento e recuperação mostrou comportamento útil e coerente:
um agente novo recuperou motivos que não estavam no código, respeitou a decisão
vigente e manteve uma lacuna como lacuna. Porém, o ADR equivalente entregou o
mesmo conhecimento com menos passos nesta rodada. Isso demonstra o valor de
preservar motivos, sem demonstrar superioridade do Xemnas sobre documentação
simples, ganho de tempo ou disposição de um usuário humano para revisar a fila.

Eu continuaria experimentando. Ainda não trataria o Xemnas como indispensável.
A próxima prova deve ser uma tarefa real com captura automática, avaliação de
candidatos e retomada por agente novo, comparada ao custo de manter um ADR.
A UI, o plugin OpenCode, a extração com provedor, a injeção automática e a
produtividade em desenvolvimento continuam sem validação nesta rodada.

## Repetir o laboratório

No Windows, com dependências Cargo disponíveis no cache:

```powershell
.\docs\operacao\avaliacao-agentes\preparar.ps1
# Use ExperimentDir e Binary informados pelo script, num terminal interativo:
& $binary $experimentDir
# Enter encerra. Para reabrir sem cadastrar/seedar novamente:
& $binary $experimentDir reopen
```

O script cria uma pasta nova, copia o lockfile do repositório e adapta apenas a
cópia para o pacote de laboratório. Não altera o lockfile do projeto. Não roda
a interface. Enquanto o servidor está ativo, em outro terminal:

```powershell
$env:XEMNAS_DATA_DIR = Join-Path $experimentDir 'data'
$message = @{
    jsonrpc = '2.0'; id = 1; method = 'tools/list'
} | ConvertTo-Json -Compress
$message | & .\target\debug\xemnas-mcp.exe --project (Join-Path $experimentDir 'scenario')
```

Compile `xemnas-mcp` antes, se necessário. Inicie os agentes de controle antes
de fornecer-lhes qualquer resposta ou oráculo. Copie o
[controle ADR](avaliacao-agentes/controle-adr.md) para uma cópia separada do
cenário somente na condição ADR. Os identificadores de decisões mudam a cada
execução: descubra as referências nas respostas; não reutilize as do piloto.
