# 22 — Instalação e integrações com um clique

**What to build:** Quem baixa o Xemnas instala com um comando ou um instalador e
liga o Claude Code (e o OpenCode) com um botão, sem editar JSON. Hoje o pacote
só traz o `xemnas.exe` e ligar o Claude Code é manual: copiar o `xemnas-mcp.exe`,
registrar o MCP e acrescentar os hooks ao `~/.claude/settings.json`
([MCP de leitura](../../operacao/operacao-e-referencia.md)).

**Blocked by:** nada. Adiado por decisão do usuário (05/10/2026): vem depois do
dogfood com o Claude Code.

**Status: open**

## Ordem

- [ ] **Comando `setup` no `xemnas-mcp`** (backend): `xemnas-mcp setup claude-code`
  registra o MCP e acrescenta os dois hooks sem mexer nos existentes; `--remove`
  tira só o que o Xemnas pôs. Idempotente, mostra o que muda antes de gravar,
  guarda cópia do arquivo anterior. A lógica fica num caso de uso do
  `application`, para o app chamar a mesma coisa.
- [ ] **Pacote com os dois binários e release pelo CI** (backend):
  `tools/package.ps1` inclui o `xemnas-mcp.exe` (pendência já registrada no MCP
  de leitura); uma tag publica o pacote no GitHub Releases.
- [ ] **Botão na tela de integrações** (front): Configurações → Integrações →
  Claude Code com Conectar, Desconectar e status (conectado, desatualizado,
  Claude Code não encontrado), chamando o caso de uso do `application`. O
  OpenCode ganha o mesmo modelo.
- [ ] **Assinatura e winget**: binários assinados (programas gratuitos para
  open source, como o SignPath, ou assinatura paga barata; conferir condições na
  hora), instalador e manifesto do winget. Trocar o ZIP por instalador revê a
  ADR-0002 (ticket 18 do MVP, entregue).
- [ ] **Plugin do Claude Code** (opcional, por último): hooks e MCP num plugin de
  marketplace, como vitrine; depende do app instalado.

**Aceite:** numa máquina limpa, `winget install` (ou o instalador) e um clique em
Conectar deixam o Claude Code recebendo contexto e enviando capturas, sem alerta
de binário não assinado; Desconectar devolve o `settings.json` ao que era.
