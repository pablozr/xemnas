# Avaliação crítica: operações e recuperação

## Escopo executado

Fiz chamadas JSON reais ao harness. Consultas `status`, `projects`, `inbox_filter`, `decisions`, `search`, `inbox_detail` e `decision_detail` usaram o banco de UI exatamente no caminho solicitado; não fiz gravações nele. A pasta `ops-scenario` foi criada vazia e registrada no banco isolado `ops-data/state/app.db`.

No banco descartável, `create_claim` criou uma afirmação e `retire_claim` efetivamente definiu seu fim de validade. `discover_map` e `index_documents` completaram sem erro, mas não encontraram componentes nem documentos. `diagnostics` retornou métricas e contagens estruturadas. `recover` completou com zero jobs requeued/failed e `outbox_drain` com zero itens pendentes. `job_retry` com ID desconhecido respondeu `job não encontrado`; o banco limpo não tinha um job Failed legítimo para testar uma retentativa ponta a ponta.

## O que os resultados dizem sobre o produto

`diagnostics` reúne perfil, contagens, perdas, latências e recibos recentes numa única consulta e não reporta segredos. Isso tem valor potencial sobre consultar várias tabelas e logs manualmente, sobretudo se as métricas forem explicadas ao operador e houver dados reais. As ações de domínio criaram e retiraram uma afirmação no armazenamento real, com datas normalizadas para UTC.

O cenário não tinha jobs falhados nem outbox pendente, então não prova que recovery ou retry resolvam um incidente. Também não permite julgar a experiência completa de operador: respostas de tipos sem `Serialize` aparecem como Debug porque foi uma escolha de apresentação do harness; não são uma representação da API de produção nem da UI. Da mesma forma, as mensagens curtas de IDs inexistentes vieram pelos casos de uso expostos no harness; a UI pode acrescentar contexto e orientar a ação seguinte. Não concluo que a UX final seja ruim com base nesses textos isolados.

Minha conclusão sobre valor é provisória: o agregado de diagnostics e as transições de domínio são úteis, mas esta rodada não demonstrou ganho concreto de recuperação frente a logs/SQLite/Git. Para decidir, é preciso observar uma falha real gerada pelo fluxo normal, acompanhar sua causa até a recuperação e comparar o tempo/esforço com a inspeção direta.

## Limitações do instrumento separadas

- `inbox_filter` com objeto `filters` fora do contrato documentado foi ignorado pela versão inicial e retornou `ok`; a chamada posterior com `project_id`, `statuses` e `limit` documentados funcionou. Não considero a tentativa arbitrária uma falha do produto.
- Uma versão inicial embrulhava uma ação desconhecida em `status=ok`; o harness atualizado retornou `status=unsupported` no topo. As duas respostas estão preservadas no JSONL.
- Os dumps Debug são formato de saída do harness para tipos sem `Serialize`, não avaliação da UI ou API de produção.
- Não havia job Failed nem outbox pendente no banco isolado; recovery e retry ativo continuam sem validação comportamental.
- O primeiro envio de texto acentuado falhou no transporte PowerShell; UTF-8 explícito resolveu. Não foi falha do produto.
- As notas atuais registram que check/build passaram no Windows com target compartilhado. Usei o executável indicado; não validei build por conta própria.

Os pedidos e respostas completos estão em `critic-operations.jsonl`. Não li outros relatórios de críticos e não usei GUI.

## Próximo exercício preparado, ainda sem execução

Preparei `prepare-opencode-outbox.mjs` para usar `createHttpMessageSource`, `createAdapter`, `createCaptureClient`, `createCheckpointStore` e `createOutboxWriter` reais do adapter, com a sessão informada e o diretório ripgrep do experimento. O endpoint Xemnas fica explicitamente indisponível no resolver, então o fluxo exercita a gravação real no outbox sem consultar discovery nem ler token. O script seleciona somente o turno completo mais recente usando checkpoint isolado e grava um JSONL agregado sem conteúdo, IDs da sessão ou credenciais. Como o envelope contém texto real da sessão, a execução exige `--execute` e `XEMNAS_ALLOW_RECOVERY_RUN=1`; não rodei o script enquanto aguardo a liberação do coordenador. A sintaxe foi verificada com `node --check`.

Também falta produzir um job Failed pelo caminho real usando um provider controlado; não gerei nem alterei registros SQL para simular esse estado.

## Captura real pelo adapter e outbox

Após liberação do coordenador, executei o script preparado contra a sessão OpenCode informada e o diretório ripgrep do experimento. O adapter real leu a sessão por HTTP e tentou o cliente Xemnas com um resolver controlado que retornou `null`; isso simulou indisponibilidade deliberada do endpoint e não demonstra que o desktop ou a API estivessem desligados. O adapter registrou `app-unavailable`, gravou um envelope pendente e não avançou o checkpoint. O resultado agregado foi um arquivo pendente com dois artefatos: um texto do usuário e um texto do assistente.

Validei sem imprimir o conteúdo: o envelope passa no schema do adapter, fingerprints SHA-256 conferem, o nome do arquivo corresponde ao hash da chave de idempotência e os campos de sessão/projeto correspondem aos alvos autorizados. O arquivo real que o harness precisará ingerir está em `recovery-data/outbox/pending`; o resultado agregado sanitizado está em `plugin-recovery.jsonl`. Ainda não houve drenagem nem comprovação de aceitação/deduplicação pelo servidor Xemnas; essa etapa depende do harness final do coordenador.

O exercício está limitado a um turno completo recente com dados reais da sessão. O texto capturado reside no envelope local porque é a entrada necessária para testar ingestão; os logs não contêm texto, token, ID bruto de sessão ou de projeto.
