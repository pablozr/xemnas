//! Copy for the assistant area. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings};

strings! {
    /// Aria label of the docked mascot while the panel is open.
    dock_close_aria { en: "Close the Xemnas assistant", pt: "Fechar o assistente Xemnas",
        es: "Cerrar el asistente Xemnas", fr: "Fermer l’assistant Xemnas",
        de: "Xemnas-Assistent schließen", it: "Chiudi l’assistente Xemnas",
        ja: "Xemnasアシスタントを閉じる", zh: "关闭 Xemnas 助手",
        ko: "Xemnas 어시스턴트 닫기", ru: "Закрыть ассистента Xemnas" }
    /// Aria label of the docked mascot while the panel is closed.
    dock_open_aria { en: "Open the Xemnas assistant", pt: "Abrir o assistente Xemnas",
        es: "Abrir el asistente Xemnas", fr: "Ouvrir l’assistant Xemnas",
        de: "Xemnas-Assistent öffnen", it: "Apri l’assistente Xemnas",
        ja: "Xemnasアシスタントを開く", zh: "打开 Xemnas 助手",
        ko: "Xemnas 어시스턴트 열기", ru: "Открыть ассистента Xemnas" }
    /// Tooltip of the docked mascot.
    dock_tooltip { en: "Xemnas, the project assistant", pt: "Xemnas, o assistente do projeto",
        es: "Xemnas, el asistente del proyecto", fr: "Xemnas, l’assistant du projet",
        de: "Xemnas, der Projektassistent", it: "Xemnas, l’assistente del progetto",
        ja: "Xemnas、プロジェクトのアシスタント", zh: "Xemnas，项目助手",
        ko: "Xemnas, 프로젝트 어시스턴트", ru: "Xemnas, ассистент проекта" }
    /// Aria label of the panel.
    panel_aria { en: "Xemnas assistant", pt: "Assistente Xemnas", es: "Asistente Xemnas",
        fr: "Assistant Xemnas", de: "Xemnas-Assistent", it: "Assistente Xemnas",
        ja: "Xemnasアシスタント", zh: "Xemnas 助手", ko: "Xemnas 어시스턴트",
        ru: "Ассистент Xemnas" }
    /// Aria label of the panel's close button.
    close_aria { en: "Close the assistant", pt: "Fechar o assistente", es: "Cerrar el asistente",
        fr: "Fermer l’assistant", de: "Assistent schließen", it: "Chiudi l’assistente",
        ja: "アシスタントを閉じる", zh: "关闭助手", ko: "어시스턴트 닫기",
        ru: "Закрыть ассистента" }
    /// Tooltip of the panel's close button.
    close { en: "Close", pt: "Fechar", es: "Cerrar", fr: "Fermer", de: "Schließen",
        it: "Chiudi", ja: "閉じる", zh: "关闭", ko: "닫기", ru: "Закрыть" }
    /// Line under the name in the panel header.
    panel_subtitle { en: "Nº I · project assistant", pt: "Nº I · assistente do projeto",
        es: "Nº I · asistente del proyecto", fr: "Nº I · assistant du projet",
        de: "Nº I · Projektassistent", it: "Nº I · assistente del progetto",
        ja: "Nº I · プロジェクトのアシスタント", zh: "Nº I · 项目助手",
        ko: "Nº I · 프로젝트 어시스턴트", ru: "Nº I · ассистент проекта" }
    /// Greeting when no project is selected.
    pick_project { en: "Pick a project in the sidebar and I’ll show you what it holds.",
        pt: "Escolha um projeto na lateral e eu mostro o que ele guarda.",
        es: "Elige un proyecto en la barra lateral y te muestro lo que guarda.",
        fr: "Choisissez un projet dans la barre latérale et je vous montre ce qu’il contient.",
        de: "Wähle links ein Projekt aus, dann zeige ich dir, was darin steckt.",
        it: "Scegli un progetto nella barra laterale e ti mostro cosa contiene.",
        ja: "サイドバーでプロジェクトを選ぶと、中身をお見せします。",
        zh: "在侧边栏选择一个项目，我来告诉你里面有什么。",
        ko: "사이드바에서 프로젝트를 고르면 안에 뭐가 있는지 보여 드려요.",
        ru: "Выберите проект в боковой панели, и я покажу, что в нём хранится." }
    /// Panel section label above the places it can take the person.
    take_you_to { en: "I can take you to", pt: "Posso levar você a",
        es: "Puedo llevarte a", fr: "Je peux vous emmener vers", de: "Ich bringe dich gern zu",
        it: "Posso portarti a", ja: "移動できる場所", zh: "我可以带你去",
        ko: "이동할 수 있는 곳", ru: "Могу отвести вас к" }
    /// Panel footer: the conversation is not wired yet.
    footer { en: "For now I only guide you around the app. Questions in plain language arrive when the assistant is connected to the AI provider.",
        pt: "Por enquanto eu guio pelo app. Perguntas em linguagem natural chegam quando o assistente for ligado ao provedor de IA.",
        es: "Por ahora solo te guío por la app. Las preguntas en lenguaje natural llegarán cuando el asistente se conecte al proveedor de IA.",
        fr: "Pour l’instant, je vous guide dans l’app. Les questions en langage naturel arriveront quand l’assistant sera relié au fournisseur d’IA.",
        de: "Vorerst führe ich dich nur durch die App. Fragen in natürlicher Sprache folgen, sobald der Assistent mit dem KI-Anbieter verbunden ist.",
        it: "Per ora ti guido nell’app. Le domande in linguaggio naturale arriveranno quando l’assistente sarà collegato al provider di IA.",
        ja: "今はアプリ内の案内だけをしています。自然な言葉での質問は、アシスタントがAIプロバイダーにつながった時点で使えるようになります。",
        zh: "目前我只负责在应用内引导你。等助手连接到 AI 提供商后，才能用自然语言提问。",
        ko: "지금은 앱 안에서 안내만 해요. 자연어 질문은 어시스턴트가 AI 공급자와 연결되면 사용할 수 있어요.",
        ru: "Пока я только подсказываю путь по приложению. Вопросы на естественном языке появятся, когда ассистент подключат к ИИ-провайдеру." }
    /// Route: open Review with nothing pending.
    route_open_review { en: "Open Review", pt: "Abrir a Revisão", es: "Abrir Revisión",
        fr: "Ouvrir la Revue", de: "Prüfung öffnen", it: "Apri Revisione",
        ja: "レビューを開く", zh: "打开审阅", ko: "검토 열기", ru: "Открыть проверку" }
    /// Route body: what Review is for.
    route_review_body { en: "Confirm, adjust or reject what was captured",
        pt: "Confirmar, ajustar ou rejeitar o que foi capturado",
        es: "Confirma, ajusta o rechaza lo que se capturó",
        fr: "Confirmez, ajustez ou rejetez ce qui a été capturé",
        de: "Bestätige, passe an oder lehne ab, was erfasst wurde",
        it: "Conferma, modifica o rifiuta ciò che è stato acquisito",
        ja: "キャプチャした内容を確定・調整・却下します",
        zh: "确认、调整或拒绝已捕获的内容",
        ko: "캡처한 내용을 확인하거나 조정하거나 거부해요",
        ru: "Подтвердите, измените или отклоните то, что было захвачено" }
    /// Route: read the overview.
    route_overview { en: "Read the project overview", pt: "Ler a Visão do projeto",
        es: "Leer la visión general del proyecto", fr: "Lire la vue d’ensemble du projet",
        de: "Projektübersicht lesen", it: "Leggi la panoramica del progetto",
        ja: "プロジェクトの概要を読む", zh: "阅读项目概览", ko: "프로젝트 개요 읽기",
        ru: "Прочитать обзор проекта" }
    /// Route body: what the overview holds.
    route_overview_body { en: "The summary and main flows, with sources",
        pt: "O resumo e os principais fluxos, com fontes",
        es: "El resumen y los flujos principales, con fuentes",
        fr: "Le résumé et les principaux flux, avec les sources",
        de: "Die Zusammenfassung und die wichtigsten Abläufe, mit Quellen",
        it: "Il riepilogo e i flussi principali, con le fonti",
        ja: "要約と主要なフロー（ソース付き）", zh: "摘要和主要流程，附来源",
        ko: "요약과 주요 흐름, 출처 포함", ru: "Сводка и основные потоки, с источниками" }
    /// Route: the timeline.
    route_timeline { en: "See what changed", pt: "Ver o que mudou", es: "Ver qué cambió",
        fr: "Voir ce qui a changé", de: "Änderungen ansehen", it: "Vedi cosa è cambiato",
        ja: "変更点を見る", zh: "查看有哪些变化", ko: "바뀐 내용 보기",
        ru: "Посмотреть, что изменилось" }
    /// Route body: what the timeline shows.
    route_timeline_body { en: "The timeline of decisions and rules",
        pt: "A linha do tempo das decisões e regras",
        es: "La línea de tiempo de decisiones y reglas",
        fr: "La chronologie des décisions et des règles",
        de: "Die Zeitleiste der Entscheidungen und Regeln",
        it: "La cronologia delle decisioni e delle regole",
        ja: "決定とルールのタイムライン", zh: "决策和规则的时间线",
        ko: "결정과 규칙의 타임라인", ru: "Хронология решений и правил" }
    /// Route: the map's suggestions.
    route_suggestions { en: "See the map’s suggestions", pt: "Ver as sugestões do mapa",
        es: "Ver las sugerencias del mapa", fr: "Voir les suggestions de la carte",
        de: "Vorschläge der Karte ansehen", it: "Vedi i suggerimenti della mappa",
        ja: "マップの提案を見る", zh: "查看地图的建议", ko: "맵의 제안 보기",
        ru: "Посмотреть предложения карты" }
    /// Route body: what the suggestions are.
    route_suggestions_body { en: "Relations, context and links waiting",
        pt: "Relações, contexto e vínculos à espera",
        es: "Relaciones, contexto y vínculos en espera",
        fr: "Relations, contexte et liens en attente",
        de: "Wartende Beziehungen, Kontext und Verknüpfungen",
        it: "Relazioni, contesto e collegamenti in attesa",
        ja: "待機中の関係、コンテキスト、リンク", zh: "等待处理的关系、上下文和关联",
        ko: "대기 중인 관계, 컨텍스트, 연결", ru: "Связи, контекст и ссылки в ожидании" }
    /// Route: test a task's context.
    route_test_task { en: "Test the context for a task", pt: "Testar o contexto de uma tarefa",
        es: "Probar el contexto de una tarea", fr: "Tester le contexte d’une tâche",
        de: "Kontext einer Aufgabe testen", it: "Testa il contesto di un’attività",
        ja: "タスクのコンテキストをテスト", zh: "测试任务的上下文",
        ko: "작업의 컨텍스트 테스트", ru: "Проверить контекст задачи" }
    /// Route body: what testing a task shows.
    route_test_task_body { en: "What the agent would receive for a request",
        pt: "O que o agente receberia para um pedido",
        es: "Lo que recibiría el agente para una petición",
        fr: "Ce que l’agent recevrait pour une demande",
        de: "Was der Agent für eine Anfrage erhalten würde",
        it: "Ciò che l’agente riceverebbe per una richiesta",
        ja: "リクエストに対してエージェントが受け取る内容",
        zh: "智能体针对某个请求会收到的内容",
        ko: "요청에 대해 에이전트가 받게 될 내용",
        ru: "Что получил бы агент по запросу" }
}

formats! {
    /// What the mascot says beside itself when candidates wait.
    whisper_pending(count: usize) { en: "{count} to review", pt: "{count} para revisar",
        es: "{count} por revisar", fr: "{count} à réviser", de: "{count} zu prüfen",
        it: "{count} da rivedere", ja: "レビュー待ち{count}件", zh: "{count} 项待审阅",
        ko: "검토 대기 {count}개", ru: "{count} на проверку" }
    /// Greeting when nothing waits for review.
    all_clear(project: &str) {
        en: "Nothing waiting for review in {project}. The decisions are in order.",
        pt: "Nada esperando revisão em {project}. As decisões estão em ordem.",
        es: "Nada esperando revisión en {project}. Las decisiones están en orden.",
        fr: "Rien en attente de revue dans {project}. Les décisions sont en ordre.",
        de: "In {project} wartet nichts auf Prüfung. Die Entscheidungen sind in Ordnung.",
        it: "Niente in attesa di revisione in {project}. Le decisioni sono in ordine.",
        ja: "{project}にレビュー待ちはありません。決定は整っています。",
        zh: "{project} 中没有待审阅的内容。决策都很整齐。",
        ko: "{project}에는 검토를 기다리는 게 없어요. 결정이 잘 정리돼 있어요.",
        ru: "В {project} ничего не ждёт проверки. Решения в порядке." }
    /// Greeting when exactly one candidate waits.
    pending_one(project: &str) {
        en: "There is 1 candidate waiting for your review in {project}.",
        pt: "Há 1 candidato esperando a sua revisão em {project}.",
        es: "Hay 1 candidato esperando tu revisión en {project}.",
        fr: "Un candidat attend votre revue dans {project}.",
        de: "In {project} wartet 1 Kandidat auf deine Prüfung.",
        it: "C’è 1 candidato in attesa della tua revisione in {project}.",
        ja: "{project}に、あなたのレビューを待つ候補が1件あります。",
        zh: "{project} 中有 1 个候选等待你审阅。",
        ko: "{project}에 검토를 기다리는 후보가 1개 있어요.",
        ru: "В {project} 1 кандидат ждёт вашей проверки." }
    /// Greeting when several (2 or more) candidates wait.
    pending_many(count: usize, project: &str) {
        en: "There are {count} candidates waiting for your review in {project}.",
        pt: "Há {count} candidatos esperando a sua revisão em {project}.",
        es: "Hay {count} candidatos esperando tu revisión en {project}.",
        fr: "{count} candidats attendent votre revue dans {project}.",
        de: "In {project} warten {count} Kandidaten auf deine Prüfung.",
        it: "Ci sono {count} candidati in attesa della tua revisione in {project}.",
        ja: "{project}に、あなたのレビューを待つ候補が{count}件あります。",
        zh: "{project} 中有 {count} 个候选等待你审阅。",
        ko: "{project}에 검토를 기다리는 후보가 {count}개 있어요.",
        ru: "Кандидатов, ожидающих вашей проверки в {project}: {count}." }
    /// Greeting when the queue is not known.
    following(project: &str) { en: "I’m keeping an eye on {project}.",
        pt: "Estou acompanhando {project}.", es: "Estoy siguiendo {project}.",
        fr: "Je veille sur {project}.", de: "Ich behalte {project} im Blick.",
        it: "Sto seguendo {project}.", ja: "{project}を見守っています。",
        zh: "我正在关注 {project}。", ko: "{project} 프로젝트를 지켜보고 있어요.",
        ru: "Слежу за {project}." }
    /// Route: review the waiting candidates.
    route_review_pending(pending: usize) {
        en: "Review candidates ({pending})", pt: "Revisar os {pending} candidatos",
        es: "Revisar candidatos ({pending})", fr: "Réviser les candidats ({pending})",
        de: "Kandidaten prüfen ({pending})", it: "Rivedi i candidati ({pending})",
        ja: "候補をレビュー（{pending}件）", zh: "审阅候选（{pending}）",
        ko: "후보 검토 ({pending}개)", ru: "Проверить кандидатов ({pending})" }
}
