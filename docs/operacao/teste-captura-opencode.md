# Teste manual de captura do OpenCode

Este roteiro testa a integração real; não estabelece uma decisão de arquitetura.
Escrever um Markdown isolado não importa uma decisão: o adapter captura os textos
da conversa e os diffs associados ao turno quando a sessão fica ociosa.

## Fluxo esperado

1. O plugin local recebe o evento de sessão ociosa e lê mensagens/diffs pelo
   `client` que o próprio OpenCode entrega ao plugin — vinculado à instância e ao
   diretório da sessão, com requisições em processo. Não há porta nem
   `OPENCODE_URL` a configurar no fluxo automático.
2. Envia a captura à API local do Xemnas; se o app estiver indisponível, grava na
   outbox para importação posterior.
3. O projeto precisa estar acompanhado no caminho usado pela sessão. A extração
   depende de um perfil de IA configurado e do consentimento correspondente.
4. O candidato aparece em **Revisão**. Só após confirmação passa a ser uma
   decisão confirmada. Alterar código não garante que exista uma decisão extraível.

O MCP é somente leitura: estar conectado não comprova que o plugin de captura
está carregado nem que a extração funciona.

## Plugin local

O OpenCode carrega automaticamente `~/.config/opencode/plugins/xemnas.ts`;
não é necessário adicionar esse mesmo arquivo ao array `plugin` do JSON.
Use um único export do factory, apontando para o build local:

```typescript
export { XemnasOpenCodeAdapter as Xemnas } from
  "../../../orca/projects/xemnas/adapters/opencode/dist/src/index.js";
```

Esse caminho é relativo à pasta global `plugins` nesta instalação Windows.
Para atualizar o build, execute `npm run build` em `adapters/opencode`.
Feche e reinicie o OpenCode após mudar o plugin ou seu build.

## API do OpenCode

No fluxo automático, o adapter lê a conversa pelo `client` do plugin: o OpenCode
o vincula à instância, ao diretório e à autorização corretos, e a leitura roda
em processo. Portanto **não** é preciso fixar porta nem definir `OPENCODE_URL`
para a captura funcionar, inclusive quando a TUI usa porta aleatória.

`OPENCODE_URL` (padrão `http://127.0.0.1:4096`) existe apenas como override
legado/diagnóstico: só é usada se o plugin rodar sem um client utilizável, por
exemplo em fixtures e testes. Nesse modo antigo, a instância precisa escutar na
porta indicada:

```powershell
# Somente para o caminho legado/diagnóstico: execute no diretório do projeto
# acompanhado, após fechar a instância anterior.
$env:OPENCODE_URL = "http://127.0.0.1:4096"
opencode --hostname 127.0.0.1 --port 4096
```

Se o plugin tiver o client (caso normal), ele tem precedência e essa variável é
ignorada. Em outro terminal, para o modo legado, confira:

```powershell
Invoke-RestMethod http://127.0.0.1:4096/global/health
```

Não abra apenas um `opencode serve` separado da conversa: ele seria outra
instância. Não exponha o servidor fora do loopback.

## Exemplo de decisão para a conversa

Após preparar a integração, envie este texto no OpenCode:

> Para este teste manual, decidi usar o identificador
> `XEMNAS-CAPTURA-TESTE-001` nos registros de teste, em vez de nomes livres.
> O motivo é conseguir localizar o candidato e distinguir o teste de decisões
> reais do projeto. O escopo é somente este teste; isso não muda a arquitetura
> nem impõe uma convenção ao código de produção. Registre a escolha e seu motivo.

Aguarde a resposta terminar e procure o identificador em **Revisão** no projeto.
Se o candidato aparecer, confira as evidências. Confirme somente se quiser testar
também a passagem para **Decisões**; caso contrário, rejeite o candidato de teste.

## Se nada aparecer

- Sem resposta em `/global/health`: resolva primeiro a API/porta do OpenCode.
- `messages-unavailable`: o adapter não conseguiu ler a conversa.
- `capture-rejected`: a API recusou a captura; confira projeto/caminho e o status.
- `outbox-pending`: a captura foi guardada para entrega posterior, não confirmada.
- `capture-accepted`: a captura chegou, mas isso não comprova extração nem confirmação.
- Captura recebida sem candidato: confira perfil de IA, consentimento e falhas de
  processamento no app. Não use `--demo` para este teste: ele não inicia API/workers.

Referências: [operação](operacao-e-referencia.md),
[plugins do OpenCode](https://opencode.ai/docs/plugins/) e
[servidor do OpenCode](https://opencode.ai/docs/server/).
