//! Copy for the overview area. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings};

/// Declares a count with its noun: one `fn name(n: usize) -> String` per entry.
/// Languages with a singular and a plural give `[one, other]` (Russian
/// `[one, few, many]`); Japanese, Chinese and Korean have a single form. The
/// literals name the count inline as `{n}`.
macro_rules! counted {
    ($(
        $(#[$meta:meta])*
        $name:ident($n:ident) {
            en: [$en1:literal, $en2:literal], pt: [$pt1:literal, $pt2:literal],
            es: [$es1:literal, $es2:literal], fr: [$fr1:literal, $fr2:literal],
            de: [$de1:literal, $de2:literal], it: [$it1:literal, $it2:literal],
            ja: $ja:literal, zh: $zh:literal, ko: $ko:literal,
            ru: [$ru1:literal, $ru2:literal, $ru5:literal] $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        pub fn $name($n: usize) -> String {
            use $crate::i18n::{french_one, one, russian_form, Language};
            match $crate::i18n::current() {
                Language::English => if one($n) { format!($en1) } else { format!($en2) },
                Language::Portuguese => if one($n) { format!($pt1) } else { format!($pt2) },
                Language::Spanish => if one($n) { format!($es1) } else { format!($es2) },
                Language::French => if french_one($n) { format!($fr1) } else { format!($fr2) },
                Language::German => if one($n) { format!($de1) } else { format!($de2) },
                Language::Italian => if one($n) { format!($it1) } else { format!($it2) },
                Language::Japanese => format!($ja),
                Language::Chinese => format!($zh),
                Language::Korean => format!($ko),
                Language::Russian => match russian_form($n) {
                    0 => format!($ru1),
                    1 => format!($ru2),
                    _ => format!($ru5),
                },
            }
        }
    )*};
}

strings! {
    /// What generating the overview sends to the provider (button tooltip, empty state).
    sends { en: "Writes a summary and the main flows with the configured AI provider. Sends the decisions in effect, the rules, the map names, and the titles, sections and first paragraph of the documentation; none of the code.",
        pt: "Gera um resumo e os principais fluxos com o provedor de IA configurado. Envia as decisões em vigor, as regras, os nomes do mapa e títulos, seções e o primeiro parágrafo da documentação; nada do código.",
        es: "Genera un resumen y los flujos principales con el proveedor de IA configurado. Envía las decisiones vigentes, las reglas, los nombres del mapa y los títulos, secciones y primer párrafo de la documentación; nada del código.",
        fr: "Génère un résumé et les principaux flux avec le fournisseur d’IA configuré. Envoie les décisions en vigueur, les règles, les noms de la carte, ainsi que les titres, sections et premier paragraphe de la documentation ; rien du code.",
        de: "Erstellt mit dem konfigurierten KI-Anbieter eine Zusammenfassung und die wichtigsten Abläufe. Gesendet werden die Entscheidungen in Kraft, die Regeln, die Namen der Karte sowie Titel, Abschnitte und der erste Absatz der Dokumentation; nichts vom Code.",
        it: "Genera un riepilogo e i flussi principali con il provider di IA configurato. Invia le decisioni in vigore, le regole, i nomi della mappa e i titoli, le sezioni e il primo paragrafo della documentazione; nulla del codice.",
        ja: "設定済みのAIプロバイダーで、要約と主要なフローを生成します。送信するのは、有効な決定、ルール、マップ上の名前、ドキュメントのタイトル・セクション・最初の段落で、コードは送信しません。",
        zh: "使用已配置的 AI 提供商生成摘要和主要流程。发送的内容包括生效中的决策、规则、地图中的名称，以及文档的标题、章节和第一段；不包含任何代码。",
        ko: "설정된 AI 공급자로 요약과 주요 흐름을 만들어요. 유효한 결정, 규칙, 맵의 이름, 문서의 제목·섹션·첫 문단을 보내며 코드는 보내지 않아요.",
        ru: "Создаёт сводку и основные потоки с помощью настроенного ИИ-провайдера. Отправляются действующие решения, правила, названия с карты, а также заголовки, разделы и первый абзац документации; кода среди них нет." }
    /// Tooltip of the button that opens the architecture and flows page.
    page_hint { en: "Creates an HTML file with the architecture and flows and opens it in the browser. Works offline and can be shared with anyone.",
        pt: "Gera um arquivo HTML com a arquitetura e os fluxos e o abre no navegador. Funciona offline e pode ser enviado a alguém.",
        es: "Genera un archivo HTML con la arquitectura y los flujos y lo abre en el navegador. Funciona sin conexión y se puede enviar a otra persona.",
        fr: "Crée un fichier HTML avec l’architecture et les flux et l’ouvre dans le navigateur. Fonctionne hors ligne et peut être envoyé à quelqu’un.",
        de: "Erstellt eine HTML-Datei mit der Architektur und den Abläufen und öffnet sie im Browser. Funktioniert offline und lässt sich an andere weitergeben.",
        it: "Genera un file HTML con l’architettura e i flussi e lo apre nel browser. Funziona offline e può essere inviato a qualcuno.",
        ja: "アーキテクチャとフローをまとめたHTMLファイルを作成し、ブラウザーで開きます。オフラインで動作し、そのまま誰かに送れます。",
        zh: "生成包含架构和流程的 HTML 文件，并在浏览器中打开。可离线使用，也可以发给他人。",
        ko: "아키텍처와 흐름을 담은 HTML 파일을 만들어 브라우저에서 열어요. 오프라인에서 동작하고 다른 사람에게 보낼 수도 있어요.",
        ru: "Создаёт HTML-файл с архитектурой и потоками и открывает его в браузере. Работает офлайн, и его можно отправить кому-нибудь." }
    /// Note under the summary on where the text comes from.
    provenance { en: "Written by the AI from the confirmed decisions and rules and the project documentation. Each passage shows its sources; what had no source was left out.",
        pt: "Escrita pela IA a partir das decisões e regras confirmadas e da documentação do projeto. Cada trecho mostra as fontes; o que não tinha fonte ficou de fora.",
        es: "Escrita por la IA a partir de las decisiones y reglas confirmadas y de la documentación del proyecto. Cada pasaje muestra sus fuentes; lo que no tenía fuente quedó fuera.",
        fr: "Rédigée par l’IA à partir des décisions et règles confirmées et de la documentation du projet. Chaque passage montre ses sources ; ce qui n’en avait pas a été écarté.",
        de: "Von der KI aus den bestätigten Entscheidungen und Regeln sowie der Projektdokumentation geschrieben. Jeder Abschnitt zeigt seine Quellen; was keine Quelle hatte, blieb draußen.",
        it: "Scritta dall’IA a partire dalle decisioni e dalle regole confermate e dalla documentazione del progetto. Ogni brano mostra le fonti; ciò che non ne aveva è stato escluso.",
        ja: "確定した決定とルール、プロジェクトのドキュメントをもとに、AIが書きました。各箇所に根拠となるソースを表示し、ソースのない内容は除いています。",
        zh: "由 AI 根据已确认的决策和规则以及项目文档撰写。每段都会显示来源；没有来源的内容已被剔除。",
        ko: "확인된 결정과 규칙, 프로젝트 문서를 바탕으로 AI가 썼어요. 각 부분에 출처가 표시되며, 출처가 없는 내용은 빠졌어요.",
        ru: "Написано ИИ на основе подтверждённых решений и правил и документации проекта. У каждого фрагмента показаны источники; то, у чего источника не было, не вошло." }
    /// Toast after the page opened in the browser.
    page_opened { en: "Page opened in the browser.", pt: "Página aberta no navegador.",
        es: "Página abierta en el navegador.", fr: "Page ouverte dans le navigateur.",
        de: "Seite im Browser geöffnet.", it: "Pagina aperta nel browser.",
        ja: "ページをブラウザーで開きました。", zh: "页面已在浏览器中打开。",
        ko: "페이지를 브라우저에서 열었어요.", ru: "Страница открыта в браузере." }
    /// Error banner when the page could not be written.
    page_failed { en: "Couldn’t create the page. Try again.",
        pt: "Não foi possível criar a página. Tente de novo.",
        es: "No se pudo crear la página. Reinténtalo.",
        fr: "Impossible de créer la page. Réessayez.",
        de: "Die Seite konnte nicht erstellt werden. Versuche es erneut.",
        it: "Impossibile creare la pagina. Riprova.",
        ja: "ページを作成できませんでした。もう一度お試しください。",
        zh: "无法创建页面。请重试。",
        ko: "페이지를 만들 수 없어요. 다시 시도해 주세요.",
        ru: "Не удалось создать страницу. Повторите попытку." }
    /// Toast after the overview was regenerated with no document queued.
    updated { en: "Overview updated.", pt: "Visão atualizada.", es: "Visión general actualizada.",
        fr: "Vue d’ensemble mise à jour.", de: "Übersicht aktualisiert.",
        it: "Panoramica aggiornata.", ja: "概要を更新しました。", zh: "概览已更新。",
        ko: "개요를 업데이트했어요.", ru: "Обзор обновлён." }
    /// Error banner when the stored overview could not be read.
    read_failed { en: "Couldn’t read the project overview. Try again.",
        pt: "Não foi possível ler a visão do projeto. Tente de novo.",
        es: "No se pudo leer la visión general del proyecto. Reinténtalo.",
        fr: "Impossible de lire la vue d’ensemble du projet. Réessayez.",
        de: "Die Projektübersicht konnte nicht gelesen werden. Versuche es erneut.",
        it: "Impossibile leggere la panoramica del progetto. Riprova.",
        ja: "プロジェクトの概要を読み込めませんでした。もう一度お試しください。",
        zh: "无法读取项目概览。请重试。",
        ko: "프로젝트 개요를 읽을 수 없어요. 다시 시도해 주세요.",
        ru: "Не удалось прочитать обзор проекта. Повторите попытку." }
    /// Page explanation when there are no parts or flows yet.
    page_empty { en: "No parts or flows yet. Generate the overview again when the map and decisions cover more of the project.",
        pt: "Ainda não há partes nem fluxos. Gere a visão de novo quando o mapa e as decisões cobrirem mais do projeto.",
        es: "Aún no hay partes ni flujos. Genera la visión general de nuevo cuando el mapa y las decisiones cubran más del proyecto.",
        fr: "Il n’y a pas encore de parties ni de flux. Générez de nouveau la vue d’ensemble quand la carte et les décisions couvriront plus du projet.",
        de: "Noch keine Teile oder Abläufe. Erstelle die Übersicht erneut, wenn Karte und Entscheidungen mehr vom Projekt abdecken.",
        it: "Non ci sono ancora parti né flussi. Genera di nuovo la panoramica quando la mappa e le decisioni copriranno più del progetto.",
        ja: "パートもフローもまだありません。マップと決定がプロジェクトのより多くをカバーしたら、概要をもう一度生成してください。",
        zh: "还没有部分或流程。等地图和决策覆盖更多项目内容后，再重新生成概览。",
        ko: "아직 파트도 흐름도 없어요. 맵과 결정이 프로젝트를 더 많이 다루게 되면 개요를 다시 만들어 주세요.",
        ru: "Пока нет ни частей, ни потоков. Создайте обзор заново, когда карта и решения охватят больше проекта." }
    /// Button and aria label: open the architecture and flows page.
    view_page { en: "View architecture and flows", pt: "Ver arquitetura e fluxos",
        es: "Ver arquitectura y flujos", fr: "Voir l’architecture et les flux",
        de: "Architektur und Abläufe ansehen", it: "Vedi architettura e flussi",
        ja: "アーキテクチャとフローを見る", zh: "查看架构和流程",
        ko: "아키텍처와 흐름 보기", ru: "Посмотреть архитектуру и потоки" }
    /// Section label of the architecture and flows card.
    page_section { en: "Architecture and flows", pt: "Arquitetura e fluxos",
        es: "Arquitectura y flujos", fr: "Architecture et flux",
        de: "Architektur und Abläufe", it: "Architettura e flussi",
        ja: "アーキテクチャとフロー", zh: "架构和流程",
        ko: "아키텍처와 흐름", ru: "Архитектура и потоки" }
    /// Generate button while it runs.
    generating { en: "Generating…", pt: "Gerando…", es: "Generando…", fr: "Génération…",
        de: "Wird erstellt…", it: "Generazione…", ja: "生成中…", zh: "正在生成…",
        ko: "만드는 중…", ru: "Создание…" }
    /// Generate button when an overview exists.
    update_overview { en: "Update overview", pt: "Atualizar visão", es: "Actualizar visión general",
        fr: "Mettre à jour la vue d’ensemble", de: "Übersicht aktualisieren",
        it: "Aggiorna panoramica", ja: "概要を更新", zh: "更新概览",
        ko: "개요 업데이트", ru: "Обновить обзор" }
    /// Generate button when there is no overview yet.
    generate_overview { en: "Generate overview", pt: "Gerar visão", es: "Generar visión general",
        fr: "Générer la vue d’ensemble", de: "Übersicht erstellen", it: "Genera panoramica",
        ja: "概要を生成", zh: "生成概览", ko: "개요 만들기", ru: "Создать обзор" }
    /// Page title.
    project_overview { en: "Project overview", pt: "Visão do projeto",
        es: "Visión general del proyecto", fr: "Vue d’ensemble du projet",
        de: "Projektübersicht", it: "Panoramica del progetto", ja: "プロジェクトの概要",
        zh: "项目概览", ko: "프로젝트 개요", ru: "Обзор проекта" }
    /// Section label of the summary.
    summary_section { en: "Summary", pt: "Resumo", es: "Resumen", fr: "Résumé",
        de: "Zusammenfassung", it: "Riepilogo", ja: "要約", zh: "摘要", ko: "요약",
        ru: "Сводка" }
    /// Label (icon caption) of the empty state.
    empty_label { en: "Overview", pt: "Visão", es: "Visión general", fr: "Vue d’ensemble",
        de: "Übersicht", it: "Panoramica", ja: "概要", zh: "概览", ko: "개요", ru: "Обзор" }
    /// Title of the empty state.
    empty_title { en: "A project summary, with architecture and flows",
        pt: "Um resumo do projeto, com arquitetura e fluxos",
        es: "Un resumen del proyecto, con arquitectura y flujos",
        fr: "Un résumé du projet, avec architecture et flux",
        de: "Eine Zusammenfassung des Projekts, mit Architektur und Abläufen",
        it: "Un riepilogo del progetto, con architettura e flussi",
        ja: "アーキテクチャとフローを含む、プロジェクトの要約",
        zh: "项目摘要，包含架构和流程",
        ko: "아키텍처와 흐름을 담은 프로젝트 요약",
        ru: "Сводка проекта с архитектурой и потоками" }
}

formats! {
    /// Explanation of the page card when there are parts or flows.
    page_what(parts: &str, flows: &str) {
        en: "{parts} and {flows} on one page to read at your own pace: a diagram you can explore and each flow step by step. Opens in the browser; nothing leaves the computer.",
        pt: "{parts} e {flows} em uma página para ler com calma: um diagrama que se explora e cada fluxo passo a passo. Abre no navegador; nada sai do computador.",
        es: "{parts} y {flows} en una página para leer con calma: un diagrama que se explora y cada flujo paso a paso. Se abre en el navegador; nada sale del ordenador.",
        fr: "{parts} et {flows} sur une page à lire tranquillement : un schéma à explorer et chaque flux pas à pas. S’ouvre dans le navigateur ; rien ne quitte l’ordinateur.",
        de: "{parts} und {flows} auf einer Seite zum ruhigen Lesen: ein Diagramm zum Erkunden und jeder Ablauf Schritt für Schritt. Öffnet sich im Browser; nichts verlässt den Computer.",
        it: "{parts} e {flows} in una pagina da leggere con calma: un diagramma da esplorare e ogni flusso passo dopo passo. Si apre nel browser; nulla esce dal computer.",
        ja: "{parts}と{flows}を、ゆっくり読める1ページにまとめます。探索できる図と、ステップごとのフローを収録。ブラウザーで開き、データがコンピューターの外に出ることはありません。",
        zh: "{parts}和{flows}整理在一个页面里，方便细读：可探索的图示，以及逐步展开的每个流程。在浏览器中打开；数据不会离开电脑。",
        ko: "파트와 흐름을 천천히 읽을 수 있는 한 페이지로 만들어요 ({parts} · {flows}). 탐색할 수 있는 다이어그램과 단계별 흐름이 담겨 있어요. 브라우저에서 열리며 컴퓨터 밖으로 나가는 것은 없어요.",
        ru: "{parts} и {flows} на одной странице для неспешного чтения: схема, которую можно изучать, и каждый поток по шагам. Открывается в браузере; ничего не покидает компьютер." }
    /// Aria label of a citation chip that opens a decision.
    open_decision(title: &str) { en: "Open the decision: {title}", pt: "Abrir a decisão: {title}",
        es: "Abrir la decisión: {title}", fr: "Ouvrir la décision : {title}",
        de: "Entscheidung öffnen: {title}", it: "Apri la decisione: {title}",
        ja: "決定を開く: {title}", zh: "打开决策：{title}", ko: "결정 열기: {title}",
        ru: "Открыть решение: {title}" }
    /// Tooltip of a citation chip that opens a decision.
    open_in_decisions(title: &str) { en: "{title} · open in Decisions",
        pt: "{title} · abrir em Decisões", es: "{title} · abrir en Decisiones",
        fr: "{title} · ouvrir dans Décisions", de: "{title} · in Entscheidungen öffnen",
        it: "{title} · apri in Decisioni", ja: "{title} · 決定で開く",
        zh: "{title} · 在决策中打开", ko: "{title} · 결정에서 열기",
        ru: "{title} · открыть в решениях" }
    /// Label of a document citation chip.
    cite_document(title: &str, id: &str) { en: "Document: {title} ({id})",
        pt: "Documento: {title} ({id})", es: "Documento: {title} ({id})",
        fr: "Document : {title} ({id})", de: "Dokument: {title} ({id})",
        it: "Documento: {title} ({id})", ja: "ドキュメント: {title} ({id})",
        zh: "文档：{title}（{id}）", ko: "문서: {title} ({id})", ru: "Документ: {title} ({id})" }
    /// Label of a rule citation chip.
    cite_rule(title: &str) { en: "Rule: {title}", pt: "Regra: {title}", es: "Regla: {title}",
        fr: "Règle : {title}", de: "Regel: {title}", it: "Regola: {title}",
        ja: "ルール: {title}", zh: "规则：{title}", ko: "규칙: {title}", ru: "Правило: {title}" }
    /// When the overview was written and from what (two kinds of source).
    generated_from_two(date: &str, first: &str, second: &str) {
        en: "Generated on {date} from {first} and {second}",
        pt: "Gerada em {date} a partir de {first} e {second}",
        es: "Generada el {date} a partir de {first} y {second}",
        fr: "Générée le {date} à partir de {first} et {second}",
        de: "Erstellt am {date} aus {first} und {second}",
        it: "Generata il {date} a partire da {first} e {second}",
        ja: "{date}に生成。元になったのは{first}と{second}",
        zh: "生成于 {date}，依据 {first}和{second}",
        ko: "{date}에 만들었어요. 바탕: {first}, {second}",
        ru: "Создан {date} на основе: {first} и {second}" }
    /// When the overview was written and from what (three kinds of source).
    generated_from_three(date: &str, first: &str, second: &str, third: &str) {
        en: "Generated on {date} from {first}, {second} and {third}",
        pt: "Gerada em {date} a partir de {first}, {second} e {third}",
        es: "Generada el {date} a partir de {first}, {second} y {third}",
        fr: "Générée le {date} à partir de {first}, {second} et {third}",
        de: "Erstellt am {date} aus {first}, {second} und {third}",
        it: "Generata il {date} a partire da {first}, {second} e {third}",
        ja: "{date}に生成。元になったのは{first}、{second}、{third}",
        zh: "生成于 {date}，依据 {first}、{second}和{third}",
        ko: "{date}에 만들었어요. 바탕: {first}, {second}, {third}",
        ru: "Создан {date} на основе: {first}, {second} и {third}" }
    /// Warning that decisions arrived since the overview was written.
    stale(new: &str) {
        en: "{new} since then. Update to bring the overview up to date.",
        pt: "{new} desde então. Atualize para incluí-las.",
        es: "{new} desde entonces. Actualiza para poner la visión general al día.",
        fr: "{new} depuis. Mettez à jour pour actualiser la vue d’ensemble.",
        de: "{new} seitdem. Aktualisiere, um die Übersicht auf den neuesten Stand zu bringen.",
        it: "{new} da allora. Aggiorna per allineare la panoramica.",
        ja: "それ以降に{new}が追加されています。更新すると反映されます。",
        zh: "此后新增了{new}。更新即可纳入。",
        ko: "그동안 {new}가 생겼어요. 업데이트하면 반영돼요.",
        ru: "С тех пор: {new}. Обновите, чтобы учесть." }
}

counted! {
    /// `3 parts`.
    parts_count(n) {
        en: ["{n} part", "{n} parts"], pt: ["{n} parte", "{n} partes"],
        es: ["{n} parte", "{n} partes"], fr: ["{n} partie", "{n} parties"],
        de: ["{n} Teil", "{n} Teile"], it: ["{n} parte", "{n} parti"],
        ja: "{n}パート", zh: "{n} 个部分", ko: "{n}개 파트",
        ru: ["{n} часть", "{n} части", "{n} частей"] }
    /// `2 flows`.
    flows_count(n) {
        en: ["{n} flow", "{n} flows"], pt: ["{n} fluxo", "{n} fluxos"],
        es: ["{n} flujo", "{n} flujos"], fr: ["{n} flux", "{n} flux"],
        de: ["{n} Ablauf", "{n} Abläufe"], it: ["{n} flusso", "{n} flussi"],
        ja: "{n}件のフロー", zh: "{n} 个流程", ko: "{n}개 흐름",
        ru: ["{n} поток", "{n} потока", "{n} потоков"] }
    /// `5 decisions`.
    decisions_count(n) {
        en: ["{n} decision", "{n} decisions"], pt: ["{n} decisão", "{n} decisões"],
        es: ["{n} decisión", "{n} decisiones"], fr: ["{n} décision", "{n} décisions"],
        de: ["{n} Entscheidung", "{n} Entscheidungen"], it: ["{n} decisione", "{n} decisioni"],
        ja: "{n}件の決定", zh: "{n} 项决策", ko: "{n}개 결정",
        ru: ["{n} решение", "{n} решения", "{n} решений"] }
    /// `4 rules`.
    rules_count(n) {
        en: ["{n} rule", "{n} rules"], pt: ["{n} regra", "{n} regras"],
        es: ["{n} regla", "{n} reglas"], fr: ["{n} règle", "{n} règles"],
        de: ["{n} Regel", "{n} Regeln"], it: ["{n} regola", "{n} regole"],
        ja: "{n}件のルール", zh: "{n} 条规则", ko: "{n}개 규칙",
        ru: ["{n} правило", "{n} правила", "{n} правил"] }
    /// `6 documents`.
    documents_count(n) {
        en: ["{n} document", "{n} documents"], pt: ["{n} documento", "{n} documentos"],
        es: ["{n} documento", "{n} documentos"], fr: ["{n} document", "{n} documents"],
        de: ["{n} Dokument", "{n} Dokumente"], it: ["{n} documento", "{n} documenti"],
        ja: "{n}件のドキュメント", zh: "{n} 份文档", ko: "{n}개 문서",
        ru: ["{n} документ", "{n} документа", "{n} документов"] }
    /// `1 new decision`: what arrived since the overview was written.
    new_decisions_count(n) {
        en: ["{n} new decision", "{n} new decisions"],
        pt: ["{n} decisão nova", "{n} decisões novas"],
        es: ["{n} decisión nueva", "{n} decisiones nuevas"],
        fr: ["{n} nouvelle décision", "{n} nouvelles décisions"],
        de: ["{n} neue Entscheidung", "{n} neue Entscheidungen"],
        it: ["{n} nuova decisione", "{n} nuove decisioni"],
        ja: "新しい決定{n}件", zh: "{n} 项新决策", ko: "새 결정 {n}개",
        ru: ["{n} новое решение", "{n} новых решения", "{n} новых решений"] }
    /// Toast after regenerating when `n` (at least 1) documents went to analysis.
    updated_queued(n) {
        en: ["Overview updated. {n} document went to analysis; candidates appear in Review.",
            "Overview updated. {n} documents went to analysis; candidates appear in Review."],
        pt: ["Visão atualizada. {n} documento foi para análise; os candidatos aparecem na Revisão.",
            "Visão atualizada. {n} documentos foram para análise; os candidatos aparecem na Revisão."],
        es: ["Visión general actualizada. {n} documento pasó a análisis; los candidatos aparecen en Revisión.",
            "Visión general actualizada. {n} documentos pasaron a análisis; los candidatos aparecen en Revisión."],
        fr: ["Vue d’ensemble mise à jour. {n} document est passé en analyse ; les candidats apparaissent dans Revue.",
            "Vue d’ensemble mise à jour. {n} documents sont passés en analyse ; les candidats apparaissent dans Revue."],
        de: ["Übersicht aktualisiert. {n} Dokument wurde zur Analyse gesendet; die Kandidaten erscheinen in der Prüfung.",
            "Übersicht aktualisiert. {n} Dokumente wurden zur Analyse gesendet; die Kandidaten erscheinen in der Prüfung."],
        it: ["Panoramica aggiornata. {n} documento è passato all’analisi; i candidati compaiono in Revisione.",
            "Panoramica aggiornata. {n} documenti sono passati all’analisi; i candidati compaiono in Revisione."],
        ja: "概要を更新しました。{n}件のドキュメントを分析に回しました。候補はレビューに表示されます。",
        zh: "概览已更新。{n} 份文档已送去分析；候选会显示在审阅中。",
        ko: "개요를 업데이트했어요. 문서 {n}개를 분석으로 보냈어요. 후보는 검토에 표시돼요.",
        ru: ["Обзор обновлён. {n} документ отправлен на анализ; кандидаты появятся в проверке.",
            "Обзор обновлён. {n} документа отправлены на анализ; кандидаты появятся в проверке.",
            "Обзор обновлён. {n} документов отправлены на анализ; кандидаты появятся в проверке."] }
}
