# ADR-0002: Distribuição Windows via ZIP + scripts PowerShell (sem instalador MSI/Inno)

- **Status:** aceito
- **Data:** 2026-09-29
- **Contexto:** Gate 5 (ticket 18) exige "build/instalador reproduzível" e "instalação limpa" para o MVP Windows-only. Opções: MSI (WiX/cargo-wix), Inno Setup, MSIX, ou ZIP + scripts. O repositório já depende de PowerShell para E2E e usa apenas toolchain Rust estável + GitHub Actions.
- **Decisão:** distribuir como **ZIP versionado** gerado por `tools/package.ps1` (build `--locked`, staging verificado por hash SHA-256) e provar instalação/desinstalação limpas com `tests/e2e/install-clean.ps1`. CI ganha job `package` que publica o ZIP como artifact.
- **Consequências:**
  - zero dependências novas de build (sem WiX/Inno/MSIX no toolchain);
  - "instalação limpa" = extrair + executar (data dir isolável por `XEMNAS_DATA_DIR`), "desinstalação" = remover diretórios — sem chaves de registro;
  - reprodutibilidade com escopo honesto: `--locked`, versão única de origem e caminho idêntico no CI, mas **sem promessa bit-a-bit** (timestamps de ZIP e metadados de build do Windows);
  - integração futura mais profunda (atalho de Menu Iniciar nativo, desinstalador no Painel, atualizador automático) exigiria migrar para MSI/MSIX — reconsiderar quando distribuição for além de anexo de CI.
- **Alternativas rejeitadas:** MSI via WiX/cargo-wix (toolchain nova e pesos de manutenção para um MVP local); Inno Setup (ferramenta externa não versionada no repo); MSIX (requisitos de empacotamento/certificação desproporcionais ao uso single-user).
