//! Copy for the app area: the window shell, sidebar, command palette and
//! window controls. See [`crate::i18n`] for how entries are declared.

use super::common::counted;
use super::{formats, strings};

strings! {
    /// Destination: Review.
    nav_review { en: "Review", pt: "Revisão", es: "Revisión", fr: "Revue", de: "Prüfung",
        it: "Revisione", ja: "レビュー", zh: "审阅", ko: "검토", ru: "Проверка" }
    /// Destination: Decisions (also a palette group).
    nav_decisions { en: "Decisions", pt: "Decisões", es: "Decisiones", fr: "Décisions",
        de: "Entscheidungen", it: "Decisioni", ja: "決定", zh: "决策", ko: "결정",
        ru: "Решения" }
    /// Destination: Context.
    nav_context { en: "Context", pt: "Contexto", es: "Contexto", fr: "Contexte",
        de: "Kontext", it: "Contesto", ja: "コンテキスト", zh: "上下文", ko: "컨텍스트",
        ru: "Контекст" }
    /// Destination: Map.
    nav_map { en: "Map", pt: "Mapa", es: "Mapa", fr: "Carte", de: "Karte", it: "Mappa",
        ja: "マップ", zh: "地图", ko: "맵", ru: "Карта" }
    /// Destination: Overview.
    nav_overview { en: "Overview", pt: "Visão", es: "Visión general", fr: "Vue d’ensemble",
        de: "Übersicht", it: "Panoramica", ja: "概要", zh: "概览", ko: "개요", ru: "Обзор" }
    /// Placeholder of the candidate filter.
    filter_candidates { en: "Filter loaded candidates", pt: "Filtrar candidatos carregados",
        es: "Filtrar candidatos cargados", fr: "Filtrer les candidats chargés",
        de: "Geladene Kandidaten filtern", it: "Filtra i candidati caricati",
        ja: "読み込み済みの候補を絞り込む", zh: "筛选已加载的候选",
        ko: "불러온 후보 필터링", ru: "Фильтр по загруженным кандидатам" }
    /// Sidebar status: captures are arriving.
    capture_active { en: "Capture active", pt: "Captura ativa", es: "Captura activa",
        fr: "Capture active", de: "Erfassung aktiv", it: "Acquisizione attiva",
        ja: "キャプチャ有効", zh: "捕获已启用", ko: "캡처 활성", ru: "Захват активен" }
    /// Tooltip of the active capture status.
    capture_active_hint { en: "The app receives OpenCode captures through the local API.",
        pt: "O app recebe capturas do OpenCode pela API local.",
        es: "La app recibe capturas de OpenCode por la API local.",
        fr: "L’app reçoit les captures d’OpenCode via l’API locale.",
        de: "Die App empfängt Erfassungen von OpenCode über die lokale API.",
        it: "L’app riceve le acquisizioni di OpenCode tramite l’API locale.",
        ja: "アプリはローカルAPI経由でOpenCodeのキャプチャを受け取ります。",
        zh: "应用通过本地 API 接收来自 OpenCode 的捕获。",
        ko: "앱이 로컬 API로 OpenCode 캡처를 받아요.",
        ru: "Приложение получает захваты OpenCode через локальный API." }
    /// Sidebar status: the local API did not start.
    capture_unavailable { en: "Capture unavailable", pt: "Captura indisponível",
        es: "Captura no disponible", fr: "Capture indisponible",
        de: "Erfassung nicht verfügbar", it: "Acquisizione non disponibile",
        ja: "キャプチャ利用不可", zh: "捕获不可用", ko: "캡처 사용 불가",
        ru: "Захват недоступен" }
    /// Tooltip of the unavailable capture status.
    capture_unavailable_hint {
        en: "The local API did not start; captures wait in the adapter’s outbox.",
        pt: "A API local não iniciou; as capturas aguardam na outbox do adapter.",
        es: "La API local no arrancó; las capturas esperan en la bandeja de salida del adaptador.",
        fr: "L’API locale n’a pas démarré ; les captures attendent dans l’outbox de l’adaptateur.",
        de: "Die lokale API ist nicht gestartet; Erfassungen warten in der Outbox des Adapters.",
        it: "L’API locale non si è avviata; le acquisizioni restano nella outbox dell’adapter.",
        ja: "ローカルAPIが起動しませんでした。キャプチャはアダプターのアウトボックスで待機します。",
        zh: "本地 API 未启动；捕获将留在适配器的发件箱中等待。",
        ko: "로컬 API가 시작되지 않았어요. 캡처는 어댑터의 아웃박스에서 기다려요.",
        ru: "Локальный API не запустился; захваты ждут в исходящих адаптера." }
    /// Sidebar status: demo data.
    demo_status { en: "Demo", pt: "Demonstração", es: "Demostración", fr: "Démonstration",
        de: "Demo", it: "Demo", ja: "デモ", zh: "演示", ko: "데모", ru: "Демонстрация" }
    /// Tooltip of the demo status.
    demo_status_hint { en: "Fictional in-memory data; no integration runs.",
        pt: "Dados fictícios em memória; nenhuma integração roda.",
        es: "Datos ficticios en memoria; no se ejecuta ninguna integración.",
        fr: "Données fictives en mémoire ; aucune intégration ne tourne.",
        de: "Fiktive Daten im Arbeitsspeicher; keine Integration läuft.",
        it: "Dati fittizi in memoria; nessuna integrazione in esecuzione.",
        ja: "メモリ上の架空のデータです。連携は動作しません。",
        zh: "内存中的虚构数据；不运行任何集成。",
        ko: "메모리의 가상 데이터예요. 연동은 실행되지 않아요.",
        ru: "Вымышленные данные в памяти; интеграции не работают." }
    /// Review tab: candidate decisions.
    tab_proposed_decisions { en: "Proposed decisions", pt: "Decisões propostas",
        es: "Decisiones propuestas", fr: "Décisions proposées",
        de: "Vorgeschlagene Entscheidungen", it: "Decisioni proposte",
        ja: "提案された決定", zh: "建议的决策", ko: "제안된 결정",
        ru: "Предложенные решения" }
    /// Review tab: suggested links.
    tab_suggested_links { en: "Suggested links", pt: "Ligações sugeridas",
        es: "Vínculos sugeridos", fr: "Liens suggérés", de: "Vorgeschlagene Verknüpfungen",
        it: "Collegamenti suggeriti", ja: "提案されたリンク", zh: "建议的关联",
        ko: "제안된 연결", ru: "Предложенные ссылки" }
    /// Placeholder of the command palette.
    palette_placeholder {
        en: "Go to a project, candidate, decision or action…",
        pt: "Ir para projeto, candidato, decisão ou ação…",
        es: "Ir a un proyecto, candidato, decisión o acción…",
        fr: "Aller à un projet, candidat, décision ou action…",
        de: "Zu Projekt, Kandidat, Entscheidung oder Aktion springen…",
        it: "Vai a un progetto, candidato, decisione o azione…",
        ja: "プロジェクト、候補、決定、操作に移動…",
        zh: "前往项目、候选、决策或操作…",
        ko: "프로젝트, 후보, 결정, 동작으로 이동…",
        ru: "Перейти к проекту, кандидату, решению или действию…" }
    /// Palette group: navigation.
    group_go_to { en: "Go to", pt: "Ir para", es: "Ir a", fr: "Aller à", de: "Gehe zu",
        it: "Vai a", ja: "移動", zh: "前往", ko: "이동", ru: "Перейти" }
    /// Palette group: candidates.
    group_candidates { en: "Candidates", pt: "Candidatos", es: "Candidatos",
        fr: "Candidats", de: "Kandidaten", it: "Candidati", ja: "候補", zh: "候选",
        ko: "후보", ru: "Кандидаты" }
    /// Palette group and item: decisions by part.
    decisions_by_part { en: "Decisions by part", pt: "Decisões por parte",
        es: "Decisiones por parte", fr: "Décisions par partie",
        de: "Entscheidungen nach Teil", it: "Decisioni per parte", ja: "パート別の決定",
        zh: "按部分查看决策", ko: "파트별 결정", ru: "Решения по частям" }
    /// Detail line of the decisions-by-part item.
    decisions_by_part_detail { en: "Decisions · filter", pt: "Decisões · filtro",
        es: "Decisiones · filtro", fr: "Décisions · filtre", de: "Entscheidungen · Filter",
        it: "Decisioni · filtro", ja: "決定 · フィルター", zh: "决策 · 筛选",
        ko: "결정 · 필터", ru: "Решения · фильтр" }
    /// Palette group: projects.
    group_projects { en: "Projects", pt: "Projetos", es: "Proyectos", fr: "Projets",
        de: "Projekte", it: "Progetti", ja: "プロジェクト", zh: "项目", ko: "프로젝트",
        ru: "Проекты" }
    /// Palette group: actions.
    group_actions { en: "Actions", pt: "Ações", es: "Acciones", fr: "Actions",
        de: "Aktionen", it: "Azioni", ja: "操作", zh: "操作", ko: "동작", ru: "Действия" }
    /// Settings: palette group, palette item and button.
    settings { en: "Settings", pt: "Configurações", es: "Ajustes", fr: "Réglages",
        de: "Einstellungen", it: "Impostazioni", ja: "設定", zh: "设置", ko: "설정",
        ru: "Настройки" }
    /// Tooltip of the settings button while settings are open.
    settings_close { en: "Close settings", pt: "Fechar configurações", es: "Cerrar ajustes",
        fr: "Fermer les réglages", de: "Einstellungen schließen",
        it: "Chiudi le impostazioni", ja: "設定を閉じる", zh: "关闭设置", ko: "설정 닫기",
        ru: "Закрыть настройки" }
    /// Palette item: review knowledge.
    review_knowledge { en: "Review knowledge", pt: "Revisar conhecimento",
        es: "Revisar conocimiento", fr: "Revoir les connaissances", de: "Wissen prüfen",
        it: "Rivedi la conoscenza", ja: "ナレッジをレビュー", zh: "审阅知识",
        ko: "지식 검토", ru: "Проверить знания" }
    /// Detail line of the review-knowledge item.
    review_knowledge_detail { en: "Context · advisory", pt: "Contexto · consultivo",
        es: "Contexto · consultivo", fr: "Contexte · consultatif",
        de: "Kontext · beratend", it: "Contesto · consultivo", ja: "コンテキスト · 参考",
        zh: "上下文 · 咨询性", ko: "컨텍스트 · 참고용", ru: "Контекст · рекомендательный" }
    /// Palette item and tooltip: project properties.
    project_properties { en: "Project properties", pt: "Propriedades do projeto",
        es: "Propiedades del proyecto", fr: "Propriétés du projet",
        de: "Projekteigenschaften", it: "Proprietà del progetto",
        ja: "プロジェクトのプロパティ", zh: "项目属性", ko: "프로젝트 속성",
        ru: "Свойства проекта" }
    /// Palette item: open a folder.
    open_folder { en: "Open folder…", pt: "Abrir pasta…", es: "Abrir carpeta…",
        fr: "Ouvrir un dossier…", de: "Ordner öffnen…", it: "Apri cartella…",
        ja: "フォルダーを開く…", zh: "打开文件夹…", ko: "폴더 열기…", ru: "Открыть папку…" }
    /// Detail line of the open-folder item.
    open_folder_detail { en: "Track a new project", pt: "Acompanhar um novo projeto",
        es: "Seguir un proyecto nuevo", fr: "Suivre un nouveau projet",
        de: "Ein neues Projekt verfolgen", it: "Segui un nuovo progetto",
        ja: "新しいプロジェクトを追跡", zh: "跟踪新项目", ko: "새 프로젝트 추적",
        ru: "Отслеживать новый проект" }
    /// Palette item: show the sidebar again.
    show_sidebar { en: "Show the sidebar", pt: "Mostrar a lateral",
        es: "Mostrar la barra lateral", fr: "Afficher la barre latérale",
        de: "Seitenleiste anzeigen", it: "Mostra la barra laterale",
        ja: "サイドバーを表示", zh: "显示侧边栏", ko: "사이드바 표시",
        ru: "Показать боковую панель" }
    /// Palette item: focus mode.
    focus_mode { en: "Focus mode: hide the sidebar", pt: "Modo foco: esconder a lateral",
        es: "Modo enfoque: ocultar la barra lateral",
        fr: "Mode focus : masquer la barre latérale",
        de: "Fokusmodus: Seitenleiste ausblenden",
        it: "Modalità focus: nascondi la barra laterale",
        ja: "集中モード: サイドバーを隠す", zh: "专注模式：隐藏侧边栏",
        ko: "집중 모드: 사이드바 숨기기", ru: "Режим фокуса: скрыть боковую панель" }
    /// Palette item: open the assistant.
    talk_to_assistant { en: "Talk to Xemnas, the assistant",
        pt: "Falar com o Xemnas, o assistente", es: "Hablar con Xemnas, el asistente",
        fr: "Parler à Xemnas, l’assistant", de: "Mit Xemnas, dem Assistenten, sprechen",
        it: "Parla con Xemnas, l’assistente", ja: "アシスタントのXemnasに話しかける",
        zh: "与助手 Xemnas 对话", ko: "어시스턴트 Xemnas와 대화",
        ru: "Поговорить с Xemnas, ассистентом" }
    /// Palette item: test the OpenCode connection.
    test_opencode { en: "Test the OpenCode connection",
        pt: "Testar conexão com o OpenCode", es: "Probar la conexión con OpenCode",
        fr: "Tester la connexion à OpenCode", de: "OpenCode-Verbindung testen",
        it: "Testa la connessione a OpenCode", ja: "OpenCodeとの接続をテスト",
        zh: "测试与 OpenCode 的连接", ko: "OpenCode 연결 테스트",
        ru: "Проверить подключение к OpenCode" }
    /// Palette item: diagnostics.
    diagnostics_tasks { en: "Diagnostics and tasks", pt: "Diagnóstico e tarefas",
        es: "Diagnóstico y tareas", fr: "Diagnostic et tâches", de: "Diagnose und Aufgaben",
        it: "Diagnostica e attività", ja: "診断とタスク", zh: "诊断与任务",
        ko: "진단 및 작업", ru: "Диагностика и задачи" }
    /// Palette item: AI and privacy.
    ai_privacy { en: "AI and privacy", pt: "IA e privacidade", es: "IA y privacidad",
        fr: "IA et confidentialité", de: "KI und Datenschutz", it: "IA e privacy",
        ja: "AIとプライバシー", zh: "AI 与隐私", ko: "AI와 개인정보",
        ru: "ИИ и конфиденциальность" }
    /// Palette item: theme and background.
    theme_and_background { en: "Theme and background", pt: "Tema e fundo",
        es: "Tema y fondo", fr: "Thème et arrière-plan", de: "Design und Hintergrund",
        it: "Tema e sfondo", ja: "テーマと背景", zh: "主题与背景", ko: "테마와 배경",
        ru: "Тема и фон" }
    /// Settings section name: diagnostics.
    section_diagnostics { en: "Diagnostics", pt: "Diagnóstico", es: "Diagnóstico",
        fr: "Diagnostic", de: "Diagnose", it: "Diagnostica", ja: "診断", zh: "诊断",
        ko: "진단", ru: "Диагностика" }
    /// Settings section name: extraction, key and sending.
    section_ai { en: "Extraction, key and sending", pt: "Extração, chave e envio",
        es: "Extracción, clave y envío", fr: "Extraction, clé et envoi",
        de: "Extraktion, Schlüssel und Versand", it: "Estrazione, chiave e invio",
        ja: "抽出、キー、送信", zh: "提取、密钥与发送", ko: "추출, 키, 전송",
        ru: "Извлечение, ключ и отправка" }
    /// Settings section name: appearance.
    section_appearance { en: "Appearance", pt: "Aparência", es: "Apariencia",
        fr: "Apparence", de: "Erscheinungsbild", it: "Aspetto", ja: "外観", zh: "外观",
        ko: "모양", ru: "Внешний вид" }
    /// Empty state of the palette.
    palette_empty { en: "Nothing matches. The palette searches what is already loaded.",
        pt: "Nada corresponde. A paleta procura no que já está carregado.",
        es: "Nada coincide. La paleta busca en lo que ya está cargado.",
        fr: "Aucun résultat. La palette cherche dans ce qui est déjà chargé.",
        de: "Keine Treffer. Die Palette durchsucht, was bereits geladen ist.",
        it: "Nessun risultato. La palette cerca in ciò che è già caricato.",
        ja: "一致するものがありません。パレットは読み込み済みの内容から探します。",
        zh: "没有匹配项。面板只在已加载的内容中搜索。",
        ko: "일치하는 항목이 없어요. 팔레트는 이미 불러온 내용에서 찾아요.",
        ru: "Ничего не найдено. Палитра ищет среди уже загруженного." }
    /// Palette footer: arrow keys.
    hint_navigate { en: "Navigate", pt: "Navegar", es: "Navegar", fr: "Naviguer",
        de: "Navigieren", it: "Naviga", ja: "移動", zh: "导航", ko: "이동", ru: "Навигация" }
    /// Palette footer: enter key.
    hint_open { en: "Open", pt: "Abrir", es: "Abrir", fr: "Ouvrir", de: "Öffnen",
        it: "Apri", ja: "開く", zh: "打开", ko: "열기", ru: "Открыть" }
    /// Palette footer and window control: close.
    close { en: "Close", pt: "Fechar", es: "Cerrar", fr: "Fermer", de: "Schließen",
        it: "Chiudi", ja: "閉じる", zh: "关闭", ko: "닫기", ru: "Закрыть" }
    /// Window control: minimize.
    minimize { en: "Minimize", pt: "Minimizar", es: "Minimizar", fr: "Réduire",
        de: "Minimieren", it: "Riduci a icona", ja: "最小化", zh: "最小化", ko: "최소화",
        ru: "Свернуть" }
    /// Window control: maximize.
    maximize { en: "Maximize", pt: "Maximizar", es: "Maximizar", fr: "Agrandir",
        de: "Maximieren", it: "Ingrandisci", ja: "最大化", zh: "最大化", ko: "최대화",
        ru: "Развернуть" }
    /// Window control: restore.
    restore { en: "Restore", pt: "Restaurar", es: "Restaurar", fr: "Restaurer",
        de: "Wiederherstellen", it: "Ripristina", ja: "元に戻す", zh: "还原", ko: "복원",
        ru: "Восстановить" }
    /// Accessible name of the theme menu.
    theme_menu { en: "Theme", pt: "Tema", es: "Tema", fr: "Thème", de: "Design",
        it: "Tema", ja: "テーマ", zh: "主题", ko: "테마", ru: "Тема" }
    /// Last row of the theme menu.
    background_more { en: "Background and more options…", pt: "Fundo e mais opções…",
        es: "Fondo y más opciones…", fr: "Arrière-plan et plus d’options…",
        de: "Hintergrund und weitere Optionen…", it: "Sfondo e altre opzioni…",
        ja: "背景とその他のオプション…", zh: "背景与更多选项…",
        ko: "배경 및 기타 옵션…", ru: "Фон и другие параметры…" }
    /// Accessible name of the last row of the theme menu.
    background_more_aria { en: "Background and more appearance options",
        pt: "Fundo e mais opções de aparência",
        es: "Fondo y más opciones de apariencia",
        fr: "Arrière-plan et plus d’options d’apparence",
        de: "Hintergrund und weitere Optionen zum Erscheinungsbild",
        it: "Sfondo e altre opzioni di aspetto", ja: "背景とその他の外観オプション",
        zh: "背景与更多外观选项", ko: "배경 및 기타 모양 옵션",
        ru: "Фон и другие параметры внешнего вида" }
    /// Window title while settings are open.
    window_title_settings { en: "xemnas — Settings", pt: "xemnas — Configurações",
        es: "xemnas — Ajustes", fr: "xemnas — Réglages", de: "xemnas — Einstellungen",
        it: "xemnas — Impostazioni", ja: "xemnas — 設定", zh: "xemnas — 设置",
        ko: "xemnas — 설정", ru: "xemnas — Настройки" }
    /// Window title before the first frame.
    window_title_projects { en: "xemnas — Projects", pt: "xemnas — Projetos",
        es: "xemnas — Proyectos", fr: "xemnas — Projets", de: "xemnas — Projekte",
        it: "xemnas — Progetti", ja: "xemnas — プロジェクト", zh: "xemnas — 项目",
        ko: "xemnas — 프로젝트", ru: "xemnas — Проекты" }
    /// Badge shown while the app runs on demo data.
    demo_badge { en: "Demo · fictional data", pt: "Demonstração · dados fictícios",
        es: "Demostración · datos ficticios", fr: "Démonstration · données fictives",
        de: "Demo · fiktive Daten", it: "Demo · dati fittizi",
        ja: "デモ · 架空のデータ", zh: "演示 · 虚构数据", ko: "데모 · 가상 데이터",
        ru: "Демонстрация · вымышленные данные" }
    /// Eyebrow of the startup error screen.
    startup_error_eyebrow { en: "Startup", pt: "Inicialização", es: "Inicio", fr: "Démarrage",
        de: "Start", it: "Avvio", ja: "起動", zh: "启动", ko: "시작", ru: "Запуск" }
    /// Title of the startup error when the database file cannot be opened.
    startup_error_open_title { en: "Could not open the database",
        pt: "Não foi possível abrir o banco de dados",
        es: "No se pudo abrir la base de datos",
        fr: "Impossible d’ouvrir la base de données",
        de: "Die Datenbank konnte nicht geöffnet werden",
        it: "Impossibile aprire il database",
        ja: "データベースを開けませんでした",
        zh: "无法打开数据库", ko: "데이터베이스를 열 수 없어요",
        ru: "Не удалось открыть базу данных" }
    /// What failed when the database file cannot be opened.
    startup_error_open_detail { en: "The database file could not be opened.",
        pt: "O arquivo do banco de dados não pôde ser aberto.",
        es: "No se pudo abrir el archivo de la base de datos.",
        fr: "Le fichier de la base de données n’a pas pu être ouvert.",
        de: "Die Datenbankdatei konnte nicht geöffnet werden.",
        it: "Impossibile aprire il file del database.",
        ja: "データベースファイルを開けませんでした。",
        zh: "无法打开数据库文件。", ko: "데이터베이스 파일을 열 수 없어요.",
        ru: "Не удалось открыть файл базы данных." }
    /// What to do when the database file cannot be opened.
    startup_error_open_hint {
        en: "Check that the folder is accessible and has free space, then open the app again.",
        pt: "Verifique se a pasta está acessível e tem espaço livre, e abra o app novamente.",
        es: "Comprueba que la carpeta sea accesible y tenga espacio libre, y vuelve a abrir la app.",
        fr: "Vérifiez que le dossier est accessible et dispose d’espace libre, puis rouvrez l’app.",
        de: "Prüfe, ob der Ordner erreichbar ist und freien Speicherplatz hat, und öffne die App dann erneut.",
        it: "Controlla che la cartella sia accessibile e abbia spazio libero, poi riapri l’app.",
        ja: "フォルダーにアクセスでき、空き容量があることを確認してから、アプリをもう一度開いてください。",
        zh: "请确认该文件夹可访问且有可用空间，然后重新打开应用。",
        ko: "폴더에 접근할 수 있고 여유 공간이 있는지 확인한 뒤 앱을 다시 열어 주세요.",
        ru: "Убедитесь, что папка доступна и на диске есть свободное место, затем снова откройте приложение." }
    /// Title of the startup error when the database cannot be migrated.
    startup_error_migration_title { en: "This database needs another version of Xemnas",
        pt: "Este banco de dados precisa de outra versão do Xemnas",
        es: "Esta base de datos necesita otra versión de Xemnas",
        fr: "Cette base de données nécessite une autre version de Xemnas",
        de: "Diese Datenbank braucht eine andere Xemnas-Version",
        it: "Questo database richiede un’altra versione di Xemnas",
        ja: "このデータベースには別のバージョンの Xemnas が必要です",
        zh: "此数据库需要其他版本的 Xemnas",
        ko: "이 데이터베이스는 다른 버전의 Xemnas가 필요해요",
        ru: "Для этой базы данных нужна другая версия Xemnas" }
    /// What happened when the database cannot be migrated; data is untouched.
    startup_error_migration_detail {
        en: "It was created or changed by a different version of Xemnas, and this version cannot update it. Your data was not changed.",
        pt: "Ele foi criado ou alterado por outra versão do Xemnas, e esta versão não consegue atualizá-lo. Seus dados não foram alterados.",
        es: "La creó o modificó otra versión de Xemnas y esta versión no puede actualizarla. Tus datos no se han modificado.",
        fr: "Elle a été créée ou modifiée par une autre version de Xemnas, et cette version ne peut pas la mettre à jour. Vos données n’ont pas été modifiées.",
        de: "Sie wurde von einer anderen Xemnas-Version erstellt oder geändert, und diese Version kann sie nicht aktualisieren. Deine Daten wurden nicht verändert.",
        it: "È stato creato o modificato da un’altra versione di Xemnas e questa versione non può aggiornarlo. I tuoi dati non sono stati modificati.",
        ja: "別のバージョンの Xemnas が作成または変更したため、このバージョンでは更新できません。データは変更されていません。",
        zh: "它由其他版本的 Xemnas 创建或修改，当前版本无法更新它。你的数据没有被更改。",
        ko: "다른 버전의 Xemnas가 만들었거나 변경해서 이 버전으로는 업데이트할 수 없어요. 데이터는 바뀌지 않았어요.",
        ru: "Она создана или изменена другой версией Xemnas, и эта версия не может её обновить. Ваши данные не изменены." }
    /// What to do when the database cannot be migrated.
    startup_error_migration_hint {
        en: "Open Xemnas with the version that created it, or restore a backup of the data folder.",
        pt: "Abra o Xemnas com a versão que o criou, ou restaure um backup da pasta de dados.",
        es: "Abre Xemnas con la versión que la creó, o restaura una copia de seguridad de la carpeta de datos.",
        fr: "Ouvrez Xemnas avec la version qui l’a créée, ou restaurez une sauvegarde du dossier de données.",
        de: "Öffne Xemnas mit der Version, die sie erstellt hat, oder stelle eine Sicherung des Datenordners wieder her.",
        it: "Apri Xemnas con la versione che lo ha creato, oppure ripristina un backup della cartella dei dati.",
        ja: "作成したバージョンの Xemnas で開くか、データフォルダーのバックアップを復元してください。",
        zh: "请用创建它的版本打开 Xemnas，或恢复数据文件夹的备份。",
        ko: "만든 버전의 Xemnas로 열거나 데이터 폴더의 백업을 복원해 주세요.",
        ru: "Откройте Xemnas той версией, которая её создала, или восстановите резервную копию папки данных." }
    /// Startup error action: open the data folder in the file manager.
    startup_error_open_folder { en: "Open data folder", pt: "Abrir pasta de dados",
        es: "Abrir carpeta de datos", fr: "Ouvrir le dossier de données",
        de: "Datenordner öffnen", it: "Apri cartella dei dati", ja: "データフォルダーを開く",
        zh: "打开数据文件夹", ko: "데이터 폴더 열기", ru: "Открыть папку данных" }
    /// Startup error action: copy the technical line for a bug report.
    startup_error_copy { en: "Copy details", pt: "Copiar detalhes", es: "Copiar detalles",
        fr: "Copier les détails", de: "Details kopieren", it: "Copia dettagli",
        ja: "詳細をコピー", zh: "复制详情", ko: "세부 정보 복사", ru: "Копировать подробности" }
    /// Toast after the startup error details were copied.
    startup_error_copied { en: "Details copied", pt: "Detalhes copiados",
        es: "Detalles copiados", fr: "Détails copiés", de: "Details kopiert",
        it: "Dettagli copiati", ja: "詳細をコピーしました", zh: "详情已复制",
        ko: "세부 정보를 복사했어요", ru: "Подробности скопированы" }
    /// Error when a capture cannot be reprocessed.
    reprocess_failed { en: "Could not reprocess the capture.",
        pt: "Não foi possível reprocessar a captura.",
        es: "No se pudo reprocesar la captura.", fr: "Impossible de retraiter la capture.",
        de: "Die Erfassung konnte nicht neu verarbeitet werden.",
        it: "Impossibile rielaborare l’acquisizione.",
        ja: "キャプチャを再処理できませんでした。", zh: "无法重新处理该捕获。",
        ko: "캡처를 재처리하지 못했어요.", ru: "Не удалось заново обработать захват." }
}

formats! {
    /// Accessible name of the sidebar status line.
    open_details(label: &str) {
        en: "{label}. Open details", pt: "{label}. Abrir detalhes",
        es: "{label}. Abrir detalles", fr: "{label}. Ouvrir les détails",
        de: "{label}. Details öffnen", it: "{label}. Apri i dettagli",
        ja: "{label}。詳細を開く", zh: "{label}。打开详情", ko: "{label}. 세부 정보 열기",
        ru: "{label}. Открыть подробности" }
    /// Sidebar status: extractions running.
    extracting(working: usize) {
        en: "Extracting {working}", pt: "Extraindo {working}", es: "Extrayendo {working}",
        fr: "Extraction de {working}", de: "Extrahiere {working}",
        it: "Estrazione di {working}", ja: "抽出中 {working}", zh: "正在提取 {working}",
        ko: "추출 중 {working}", ru: "Извлечение: {working}" }
    /// What is waiting on the person, next to the review tabs.
    waiting_for_you(waiting: &str) {
        en: "Waiting on you: {waiting}", pt: "Aguardando você: {waiting}",
        es: "Esperándote: {waiting}", fr: "En attente de vous : {waiting}",
        de: "Wartet auf dich: {waiting}", it: "In attesa di te: {waiting}",
        ja: "あなたの対応待ち: {waiting}", zh: "等你处理：{waiting}",
        ko: "확인 대기 중: {waiting}", ru: "Ждёт вас: {waiting}" }
    /// Palette item: decisions about one part.
    decisions_about(name: &str) {
        en: "Decisions about {name}", pt: "Decisões sobre {name}",
        es: "Decisiones sobre {name}", fr: "Décisions sur {name}",
        de: "Entscheidungen zu {name}", it: "Decisioni su {name}",
        ja: "{name}に関する決定", zh: "关于 {name} 的决策", ko: "{name}에 대한 결정",
        ru: "Решения о {name}" }
    /// Accessible name of the project breadcrumb.
    project_properties_aria(name: &str) {
        en: "{name}: project properties", pt: "{name}: propriedades do projeto",
        es: "{name}: propiedades del proyecto", fr: "{name} : propriétés du projet",
        de: "{name}: Projekteigenschaften", it: "{name}: proprietà del progetto",
        ja: "{name}：プロジェクトのプロパティ", zh: "{name}：项目属性",
        ko: "{name}: 프로젝트 속성", ru: "{name}: свойства проекта" }
    /// Palette item: switch the theme.
    use_theme(theme: &str) {
        en: "Use {theme} theme", pt: "Usar tema {theme}", es: "Usar el tema {theme}",
        fr: "Utiliser le thème {theme}", de: "Design {theme} verwenden",
        it: "Usa il tema {theme}", ja: "{theme}テーマを使う", zh: "使用{theme}主题",
        ko: "{theme} 테마 사용", ru: "Использовать тему «{theme}»" }
    /// Accessible name of the theme button.
    theme_aria(mode: &str) {
        en: "Theme: {mode}. Choose theme", pt: "Tema: {mode}. Escolher tema",
        es: "Tema: {mode}. Elegir tema", fr: "Thème : {mode}. Choisir le thème",
        de: "Design: {mode}. Design wählen", it: "Tema: {mode}. Scegli il tema",
        ja: "テーマ: {mode}。テーマを選ぶ", zh: "主题：{mode}。选择主题",
        ko: "테마: {mode}. 테마 선택", ru: "Тема: {mode}. Выбрать тему" }
    /// Tooltip of the theme button.
    theme_tooltip(mode: &str) {
        en: "Theme: {mode}", pt: "Tema: {mode}", es: "Tema: {mode}", fr: "Thème : {mode}",
        de: "Design: {mode}", it: "Tema: {mode}", ja: "テーマ: {mode}", zh: "主题：{mode}",
        ko: "테마: {mode}", ru: "Тема: {mode}" }
    /// Window title with a project open.
    window_title_project(name: &str, destination: &str) {
        en: "{name} · {destination} — xemnas", pt: "{name} · {destination} — xemnas",
        es: "{name} · {destination} — xemnas", fr: "{name} · {destination} — xemnas",
        de: "{name} · {destination} — xemnas", it: "{name} · {destination} — xemnas",
        ja: "{name} · {destination} — xemnas", zh: "{name} · {destination} — xemnas",
        ko: "{name} · {destination} — xemnas", ru: "{name} · {destination} — xemnas" }
}

/// Count of failed jobs in the sidebar status.
pub fn failures(n: usize) -> String {
    counted(
        n,
        [
            ["failure", "failures", "failures"],
            ["falha", "falhas", "falhas"],
            ["fallo", "fallos", "fallos"],
            ["échec", "échecs", "échecs"],
            ["Fehler", "Fehler", "Fehler"],
            ["errore", "errori", "errori"],
            ["件の失敗", "件の失敗", "件の失敗"],
            ["个失败", "个失败", "个失败"],
            ["건 실패", "건 실패", "건 실패"],
            ["сбой", "сбоя", "сбоев"],
        ],
    )
}

/// Count of decisions waiting in Review.
pub fn decisions_count(n: usize) -> String {
    counted(
        n,
        [
            ["decision", "decisions", "decisions"],
            ["decisão", "decisões", "decisões"],
            ["decisión", "decisiones", "decisiones"],
            ["décision", "décisions", "décisions"],
            ["Entscheidung", "Entscheidungen", "Entscheidungen"],
            ["decisione", "decisioni", "decisioni"],
            ["件の決定", "件の決定", "件の決定"],
            ["项决策", "项决策", "项决策"],
            ["개 결정", "개 결정", "개 결정"],
            ["решение", "решения", "решений"],
        ],
    )
}

/// Count of suggested links waiting in Review.
pub fn links_count(n: usize) -> String {
    counted(
        n,
        [
            ["link", "links", "links"],
            ["ligação", "ligações", "ligações"],
            ["vínculo", "vínculos", "vínculos"],
            ["lien", "liens", "liens"],
            ["Verknüpfung", "Verknüpfungen", "Verknüpfungen"],
            ["collegamento", "collegamenti", "collegamenti"],
            ["件のリンク", "件のリンク", "件のリンク"],
            ["个关联", "个关联", "个关联"],
            ["개 연결", "개 연결", "개 연결"],
            ["ссылка", "ссылки", "ссылок"],
        ],
    )
}

/// Count of decisions in effect for a part.
pub fn in_effect_count(n: usize) -> String {
    counted(
        n,
        [
            ["in effect", "in effect", "in effect"],
            ["em vigor", "em vigor", "em vigor"],
            ["vigente", "vigentes", "vigentes"],
            ["en vigueur", "en vigueur", "en vigueur"],
            ["in Kraft", "in Kraft", "in Kraft"],
            ["in vigore", "in vigore", "in vigore"],
            ["件が有効", "件が有効", "件が有効"],
            ["项生效中", "项生效中", "项生效中"],
            ["개 유효", "개 유효", "개 유효"],
            ["действует", "действуют", "действуют"],
        ],
    )
}
