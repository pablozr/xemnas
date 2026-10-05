# Idiomas da interface

Status: regra vigente para todo texto do app desktop (`apps/desktop-gpui`).

O app é oferecido em dez idiomas; o padrão é **inglês**. A pessoa troca em
Configurações › Idioma (ou pela paleta, "Change language") e a troca vale na
hora para todas as janelas, salva em `settings/appearance.json` (`"language"`).
A demo aceita `--language <tag>` (`en`, `pt-BR`, `es`, `fr`, `de`, `it`, `ja`,
`zh-CN`, `ko`, `ru`).

## Como o texto chega à tela

- Todo texto que a pessoa lê ou que um leitor de tela anuncia (`child`,
  `aria_label`, tooltip, placeholder, toast, banner, título de janela,
  diálogo de arquivo, item e grupo da paleta) vem de `crate::i18n`, um módulo
  por área (`i18n::inbox`, `i18n::settings`...). Nunca um literal na tela.
- Texto fixo: `strings!` gera `fn nome() -> &'static str`. Texto com valores:
  `formats!` gera `fn nome(args) -> String`, com os argumentos nomeados dentro
  do literal (`"{count} decisions"`), para cada idioma poder reordená-los.
  Plurais e frases que mudam de forma por idioma são funções comuns com
  `match i18n::current()`, usando `i18n::one`, `i18n::french_one` e
  `i18n::russian_form`.
- Os dois macros exigem os dez idiomas: tradução faltando é erro de
  compilação. Não existe fallback silencioso para o inglês.
- Custo: uma leitura atômica e um `match` sobre `&'static str` por texto, sem
  alocação, mapa ou arquivo lido em tempo de execução.
- Datas, tempo relativo e separador de milhar seguem o idioma
  (`i18n::format`, usado por `screens::format`).
- Não se traduz conteúdo: decisões, nomes de projeto, evidências, capturas e
  a demo são o que a pessoa (ou o agente) escreveu. Também ficam como estão
  ids de elementos, chaves salvas, logs (`tracing`) e nomes de produto
  (xemnas, OpenCode, ChatGPT, Ollama, LM Studio, MCP, Quiet Glass).
- Mensagens que chegam prontas do backend (`application`, `ai-provider`)
  ainda estão em português; traduzi-las é trabalho da sessão de backend
  (devolver um tipo de erro que a tela traduz), registrado em
  [roadmap/mvp/issues](../roadmap/mvp/issues/21-mensagens-do-backend-traduziveis.md).

## Tom

Mesma voz curta e direta do texto em português, que segue sendo a referência
de sentido. Inglês em sentence case ("Try again", não "Try Again"). Tratamento:
pt "você", es "tú", fr "vous", de "du", it "tu", ja です/ます, ko 해요체, ru "вы".
Mantém os separadores tipográficos do original (`›`, `·`, `—`, `…`).

## Glossário

Termos do produto ([CONTEXT.md](../produto/CONTEXT.md)) traduzidos sempre do
mesmo jeito em todas as telas.

| pt | en | es | fr | de | it | ja | zh | ko | ru |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Projeto | Project | Proyecto | Projet | Projekt | Progetto | プロジェクト | 项目 | 프로젝트 | Проект |
| Decisão | Decision | Decisión | Décision | Entscheidung | Decisione | 決定 | 决策 | 결정 | Решение |
| Revisão (destino) | Review | Revisión | Revue | Prüfung | Revisione | レビュー | 审阅 | 검토 | Проверка |
| Contexto | Context | Contexto | Contexte | Kontext | Contesto | コンテキスト | 上下文 | 컨텍스트 | Контекст |
| Mapa | Map | Mapa | Carte | Karte | Mappa | マップ | 地图 | 맵 | Карта |
| Visão (geral) | Overview | Visión general | Vue d’ensemble | Übersicht | Panoramica | 概要 | 概览 | 개요 | Обзор |
| Grafo | Graph | Grafo | Graphe | Graph | Grafo | グラフ | 图谱 | 그래프 | Граф |
| Captura | Capture | Captura | Capture | Erfassung | Acquisizione | キャプチャ | 捕获 | 캡처 | Захват |
| Candidato | Candidate | Candidato | Candidat | Kandidat | Candidato | 候補 | 候选 | 후보 | Кандидат |
| Evidência | Evidence | Evidencia | Preuve | Beleg | Evidenza | 根拠 | 证据 | 근거 | Доказательство |
| Regra | Rule | Regla | Règle | Regel | Regola | ルール | 规则 | 규칙 | Правило |
| Premissa | Assumption | Supuesto | Hypothèse | Annahme | Presupposto | 前提 | 前提 | 전제 | Допущение |
| Objetivo | Goal | Objetivo | Objectif | Ziel | Obiettivo | 目標 | 目标 | 목표 | Цель |
| Escopo | Scope | Alcance | Portée | Geltungsbereich | Ambito | 範囲 | 范围 | 범위 | Область |
| Em vigor / vigente | In effect | Vigente | En vigueur | In Kraft | In vigore | 有効 | 生效中 | 유효 | Действует |
| Substituída | Superseded | Sustituida | Remplacée | Ersetzt | Sostituita | 置き換え済み | 已取代 | 대체됨 | Заменено |
| Relação | Relation | Relación | Relation | Beziehung | Relazione | 関係 | 关系 | 관계 | Связь |
| Vínculo / ligação | Link | Vínculo | Lien | Verknüpfung | Collegamento | リンク | 关联 | 연결 | Ссылка |
| Sugestão | Suggestion | Sugerencia | Suggestion | Vorschlag | Suggerimento | 提案 | 建议 | 제안 | Предложение |
| Componente | Component | Componente | Composant | Komponente | Componente | コンポーネント | 组件 | 컴포넌트 | Компонент |
| Parte | Part | Parte | Partie | Teil | Parte | パート | 部分 | 파트 | Часть |
| Análise | Analysis | Análisis | Analyse | Analyse | Analisi | 分析 | 分析 | 분석 | Анализ |
| Fonte | Source | Fuente | Source | Quelle | Fonte | ソース | 来源 | 출처 | Источник |
| Documento | Document | Documento | Document | Dokument | Documento | ドキュメント | 文档 | 문서 | Документ |
| Provedor | Provider | Proveedor | Fournisseur | Anbieter | Provider | プロバイダー | 提供商 | 공급자 | Провайдер |
| Modelo | Model | Modelo | Modèle | Modell | Modello | モデル | 模型 | 모델 | Модель |
| Chave de API | API key | Clave de API | Clé d’API | API-Schlüssel | Chiave API | APIキー | API 密钥 | API 키 | API-ключ |
| Cofre do sistema | System credential store | Almacén de credenciales del sistema | Coffre d’identifiants du système | Anmeldeinformationsspeicher des Systems | Archivio credenziali di sistema | システムの資格情報ストア | 系统凭据存储 | 시스템 자격 증명 저장소 | Системное хранилище учётных данных |
| Consentimento | Consent | Consentimiento | Consentement | Einwilligung | Consenso | 同意 | 同意 | 동의 | Согласие |
| Prévia | Preview | Vista previa | Aperçu | Vorschau | Anteprima | プレビュー | 预览 | 미리 보기 | Предпросмотр |
| Extração | Extraction | Extracción | Extraction | Extraktion | Estrazione | 抽出 | 提取 | 추출 | Извлечение |
| Reprocessar | Reprocess | Reprocesar | Retraiter | Neu verarbeiten | Rielabora | 再処理 | 重新处理 | 재처리 | Обработать заново |
| Agente | Agent | Agente | Agent | Agent | Agente | エージェント | 智能体 | 에이전트 | Агент |
| Assistente | Assistant | Asistente | Assistant | Assistent | Assistente | アシスタント | 助手 | 어시스턴트 | Ассистент |
| Sessão | Session | Sesión | Session | Sitzung | Sessione | セッション | 会话 | 세션 | Сессия |
| Fila (de tarefas) | Queue | Cola | File d’attente | Warteschlange | Coda | キュー | 队列 | 대기열 | Очередь |
| Configurações | Settings | Ajustes | Réglages | Einstellungen | Impostazioni | 設定 | 设置 | 설정 | Настройки |
| Diagnóstico | Diagnostics | Diagnóstico | Diagnostic | Diagnose | Diagnostica | 診断 | 诊断 | 진단 | Диагностика |
| Aparência | Appearance | Apariencia | Apparence | Erscheinungsbild | Aspetto | 外観 | 外观 | 모양 | Внешний вид |
| Idioma | Language | Idioma | Langue | Sprache | Lingua | 言語 | 语言 | 언어 | Язык |
| Confirmar | Confirm | Confirmar | Confirmer | Bestätigen | Conferma | 確定 | 确认 | 확인 | Подтвердить |
| Rejeitar | Reject | Rechazar | Rejeter | Ablehnen | Rifiuta | 却下 | 拒绝 | 거부 | Отклонить |
| Cancelar | Cancel | Cancelar | Annuler | Abbrechen | Annulla | キャンセル | 取消 | 취소 | Отмена |
| Salvar | Save | Guardar | Enregistrer | Speichern | Salva | 保存 | 保存 | 저장 | Сохранить |
| Tentar de novo | Try again | Reintentar | Réessayer | Erneut versuchen | Riprova | 再試行 | 重试 | 다시 시도 | Повторить |
| Fechar | Close | Cerrar | Fermer | Schließen | Chiudi | 閉じる | 关闭 | 닫기 | Закрыть |
| Abrir | Open | Abrir | Ouvrir | Öffnen | Apri | 開く | 打开 | 열기 | Открыть |
| Exportar | Export | Exportar | Exporter | Exportieren | Esporta | エクスポート | 导出 | 내보내기 | Экспорт |
| Histórico | History | Historial | Historique | Verlauf | Cronologia | 履歴 | 历史 | 기록 | История |
| Versão | Version | Versión | Version | Version | Versione | バージョン | 版本 | 버전 | Версия |
| Motivo | Reason | Motivo | Raison | Grund | Motivo | 理由 | 原因 | 이유 | Причина |
