//! Copy for the context area. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings};

/// Declares a counted noun: one `fn name(n) -> String` per entry, whose
/// literals are `format!` strings naming `n` (`"{n} decision"`). English,
/// Portuguese, Spanish, German and Italian have singular and plural, French
/// treats 0 as singular, Russian has three forms and Japanese, Chinese and
/// Korean have one.
macro_rules! plurals {
    ($(
        $(#[$meta:meta])*
        $name:ident ( $n:ident ) {
            en: ($en1:literal, $en2:literal), pt: ($pt1:literal, $pt2:literal),
            es: ($es1:literal, $es2:literal), fr: ($fr1:literal, $fr2:literal),
            de: ($de1:literal, $de2:literal), it: ($it1:literal, $it2:literal),
            ja: $ja:literal, zh: $zh:literal, ko: $ko:literal,
            ru: ($ru1:literal, $ru2:literal, $ru3:literal) $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        pub fn $name($n: usize) -> String {
            use $crate::i18n::{french_one, one, russian_form, Language};
            match $crate::i18n::current() {
                Language::English if one($n) => format!($en1),
                Language::English => format!($en2),
                Language::Portuguese if one($n) => format!($pt1),
                Language::Portuguese => format!($pt2),
                Language::Spanish if one($n) => format!($es1),
                Language::Spanish => format!($es2),
                Language::French if french_one($n) => format!($fr1),
                Language::French => format!($fr2),
                Language::German if one($n) => format!($de1),
                Language::German => format!($de2),
                Language::Italian if one($n) => format!($it1),
                Language::Italian => format!($it2),
                Language::Japanese => format!($ja),
                Language::Chinese => format!($zh),
                Language::Korean => format!($ko),
                Language::Russian => match russian_form($n) {
                    0 => format!($ru1),
                    1 => format!($ru2),
                    _ => format!($ru3),
                },
            }
        }
    )*};
}

plurals! {
    /// Documents read from the project folder, in the header of the documents page.
    documents_read(n) {
        en: ("{n} document read from the project folder",
            "{n} documents read from the project folder"),
        pt: ("{n} documento lido da pasta do projeto",
            "{n} documentos lidos da pasta do projeto"),
        es: ("{n} documento leído de la carpeta del proyecto",
            "{n} documentos leídos de la carpeta del proyecto"),
        fr: ("{n} document lu dans le dossier du projet",
            "{n} documents lus dans le dossier du projet"),
        de: ("{n} Dokument aus dem Projektordner gelesen",
            "{n} Dokumente aus dem Projektordner gelesen"),
        it: ("{n} documento letto dalla cartella del progetto",
            "{n} documenti letti dalla cartella del progetto"),
        ja: "プロジェクトフォルダーから{n}件のドキュメントを読み込みました",
        zh: "已从项目文件夹读取 {n} 个文档",
        ko: "프로젝트 폴더에서 문서 {n}개를 읽었어요",
        ru: ("{n} документ прочитан из папки проекта",
            "{n} документа прочитано из папки проекта",
            "{n} документов прочитано из папки проекта")
    }
    /// Headings of a document, after its path.
    document_sections(n) {
        en: ("· {n} section", "· {n} sections"),
        pt: ("· {n} seção", "· {n} seções"),
        es: ("· {n} sección", "· {n} secciones"),
        fr: ("· {n} section", "· {n} sections"),
        de: ("· {n} Abschnitt", "· {n} Abschnitte"),
        it: ("· {n} sezione", "· {n} sezioni"),
        ja: "· {n}セクション",
        zh: "· {n} 个章节",
        ko: "· 섹션 {n}개",
        ru: ("· {n} раздел", "· {n} раздела", "· {n} разделов")
    }
    /// Decisions in the Context Pack preview.
    pack_decisions(n) {
        en: ("{n} decision", "{n} decisions"),
        pt: ("{n} decisão", "{n} decisões"),
        es: ("{n} decisión", "{n} decisiones"),
        fr: ("{n} décision", "{n} décisions"),
        de: ("{n} Entscheidung", "{n} Entscheidungen"),
        it: ("{n} decisione", "{n} decisioni"),
        ja: "決定{n}件",
        zh: "{n} 项决策",
        ko: "결정 {n}개",
        ru: ("{n} решение", "{n} решения", "{n} решений")
    }
    /// Rules in the Context Pack preview.
    pack_rules(n) {
        en: ("{n} rule", "{n} rules"),
        pt: ("{n} regra", "{n} regras"),
        es: ("{n} regla", "{n} reglas"),
        fr: ("{n} règle", "{n} règles"),
        de: ("{n} Regel", "{n} Regeln"),
        it: ("{n} regola", "{n} regole"),
        ja: "ルール{n}件",
        zh: "{n} 条规则",
        ko: "규칙 {n}개",
        ru: ("{n} правило", "{n} правила", "{n} правил")
    }
    /// Decisions in force counted in the pipeline on the overview.
    decisions_in_force(n) {
        en: ("{n} decision in effect", "{n} decisions in effect"),
        pt: ("{n} decisão em vigor", "{n} decisões em vigor"),
        es: ("{n} decisión vigente", "{n} decisiones vigentes"),
        fr: ("{n} décision en vigueur", "{n} décisions en vigueur"),
        de: ("{n} Entscheidung in Kraft", "{n} Entscheidungen in Kraft"),
        it: ("{n} decisione in vigore", "{n} decisioni in vigore"),
        ja: "有効な決定{n}件",
        zh: "{n} 项生效中的决策",
        ko: "유효한 결정 {n}개",
        ru: ("{n} действующее решение", "{n} действующих решения",
            "{n} действующих решений")
    }
    /// Project rules counted in the pipeline on the overview.
    project_rules(n) {
        en: ("{n} project rule", "{n} project rules"),
        pt: ("{n} regra do projeto", "{n} regras do projeto"),
        es: ("{n} regla del proyecto", "{n} reglas del proyecto"),
        fr: ("{n} règle du projet", "{n} règles du projet"),
        de: ("{n} Projektregel", "{n} Projektregeln"),
        it: ("{n} regola del progetto", "{n} regole del progetto"),
        ja: "プロジェクトのルール{n}件",
        zh: "{n} 条项目规则",
        ko: "프로젝트 규칙 {n}개",
        ru: ("{n} правило проекта", "{n} правила проекта", "{n} правил проекта")
    }
    /// Deliveries in one day of the history.
    deliveries_count(n) {
        en: ("{n} delivery", "{n} deliveries"),
        pt: ("{n} entrega", "{n} entregas"),
        es: ("{n} entrega", "{n} entregas"),
        fr: ("{n} livraison", "{n} livraisons"),
        de: ("{n} Lieferung", "{n} Lieferungen"),
        it: ("{n} consegna", "{n} consegne"),
        ja: "配信{n}件",
        zh: "{n} 次交付",
        ko: "전달 {n}건",
        ru: ("{n} доставка", "{n} доставки", "{n} доставок")
    }
    /// Items one delivery carried.
    items_count(n) {
        en: ("{n} item", "{n} items"),
        pt: ("{n} item", "{n} itens"),
        es: ("{n} elemento", "{n} elementos"),
        fr: ("{n} élément", "{n} éléments"),
        de: ("{n} Eintrag", "{n} Einträge"),
        it: ("{n} elemento", "{n} elementi"),
        ja: "項目{n}件",
        zh: "{n} 个条目",
        ko: "항목 {n}개",
        ru: ("{n} элемент", "{n} элемента", "{n} элементов")
    }
}

strings! {
    // Index groups and page switch.

    /// Index entry and page title: the overview.
    group_overview { en: "Overview", pt: "Visão geral", es: "Visión general",
        fr: "Vue d’ensemble", de: "Übersicht", it: "Panoramica", ja: "概要",
        zh: "概览", ko: "개요", ru: "Обзор" }
    /// Index entry: where the context comes from.
    group_sources { en: "Sources", pt: "Fontes", es: "Fuentes", fr: "Sources",
        de: "Quellen", it: "Fonti", ja: "ソース", zh: "来源", ko: "출처", ru: "Источники" }
    /// Index entry and page title: what was delivered to the agent.
    group_deliveries { en: "Deliveries", pt: "Entregas", es: "Entregas",
        fr: "Livraisons", de: "Lieferungen", it: "Consegne", ja: "配信",
        zh: "交付", ko: "전달", ru: "Доставки" }
    /// Index entry: the delivery settings.
    group_settings { en: "Settings", pt: "Ajustes", es: "Ajustes", fr: "Réglages",
        de: "Einstellungen", it: "Impostazioni", ja: "設定", zh: "设置", ko: "설정",
        ru: "Настройки" }
    /// Switch tab and page title: decisions in force.
    tab_decisions { en: "Decisions", pt: "Decisões", es: "Decisiones", fr: "Décisions",
        de: "Entscheidungen", it: "Decisioni", ja: "決定", zh: "决策", ko: "결정",
        ru: "Решения" }
    /// Switch tab and page title: the project rules.
    tab_rules { en: "Rules", pt: "Regras", es: "Reglas", fr: "Règles", de: "Regeln",
        it: "Regole", ja: "ルール", zh: "规则", ko: "규칙", ru: "Правила" }
    /// Switch tab and page title: the project documentation.
    tab_documents { en: "Documentation", pt: "Documentação", es: "Documentación",
        fr: "Documentation", de: "Dokumentation", it: "Documentazione",
        ja: "ドキュメント", zh: "文档", ko: "문서", ru: "Документация" }
    /// Switch tab: the AI knowledge review.
    tab_knowledge_review { en: "Review with AI", pt: "Revisar com IA",
        es: "Revisar con IA", fr: "Réviser avec l’IA", de: "Mit KI prüfen",
        it: "Rivedi con l’IA", ja: "AIでレビュー", zh: "用 AI 审阅",
        ko: "AI로 검토", ru: "Проверить с помощью ИИ" }
    /// Switch tab: the delivery history.
    tab_history { en: "History", pt: "Histórico", es: "Historial", fr: "Historique",
        de: "Verlauf", it: "Cronologia", ja: "履歴", zh: "历史", ko: "기록",
        ru: "История" }
    /// Switch tab and page title: try a task against the context.
    tab_test_task { en: "Test a task", pt: "Testar uma tarefa", es: "Probar una tarea",
        fr: "Tester une tâche", de: "Aufgabe testen", it: "Prova un’attività",
        ja: "タスクを試す", zh: "测试任务", ko: "작업 테스트", ru: "Проверить задачу" }
    /// Switch tab and page title: when the agent receives context.
    tab_delivery_mode { en: "Delivery mode", pt: "Modo de entrega",
        es: "Modo de entrega", fr: "Mode de livraison", de: "Lieferungsmodus",
        it: "Modalità di consegna", ja: "配信モード", zh: "交付模式",
        ko: "전달 모드", ru: "Режим доставки" }

    // Fields.

    /// Placeholder of the budget field.
    budget_placeholder { en: "Server default (300)", pt: "Padrão do servidor (300)",
        es: "Valor predeterminado del servidor (300)", fr: "Valeur par défaut du serveur (300)",
        de: "Serverstandard (300)", it: "Predefinito del server (300)",
        ja: "サーバーの既定値 (300)", zh: "服务器默认值 (300)", ko: "서버 기본값 (300)",
        ru: "По умолчанию на сервере (300)" }
    /// Placeholder of the new rule field.
    rule_placeholder { en: "E.g. every migration is forward-only",
        pt: "Ex.: toda migração é forward-only",
        es: "Ej.: toda migración es forward-only",
        fr: "Ex. : toute migration est forward-only",
        de: "Z. B. jede Migration ist forward-only",
        it: "Es.: ogni migrazione è forward-only",
        ja: "例: すべてのマイグレーションは forward-only",
        zh: "例如：所有迁移都是 forward-only",
        ko: "예: 모든 마이그레이션은 forward-only",
        ru: "Напр.: любая миграция только forward-only" }
    /// Placeholder of the task field in the preview.
    task_placeholder { en: "E.g. add CSV export of decisions",
        pt: "Ex.: adicionar exportação em CSV das decisões",
        es: "Ej.: añadir exportación a CSV de las decisiones",
        fr: "Ex. : ajouter l’export CSV des décisions",
        de: "Z. B. CSV-Export der Entscheidungen hinzufügen",
        it: "Es.: aggiungere l’esportazione in CSV delle decisioni",
        ja: "例: 決定の CSV エクスポートを追加する",
        zh: "例如：添加决策的 CSV 导出",
        ko: "예: 결정을 CSV로 내보내는 기능 추가",
        ru: "Напр.: добавить экспорт решений в CSV" }

    // Notices and errors.

    /// Toast after saving the context mode.
    mode_saved { en: "Context mode saved.", pt: "Modo de contexto salvo.",
        es: "Modo de contexto guardado.", fr: "Mode de contexte enregistré.",
        de: "Kontextmodus gespeichert.", it: "Modalità di contesto salvata.",
        ja: "コンテキストモードを保存しました。", zh: "已保存上下文模式。",
        ko: "컨텍스트 모드를 저장했어요.", ru: "Режим контекста сохранён." }
    /// Toast when the folder holds no documents.
    no_documents_found { en: "No documents found in the project folder.",
        pt: "Nenhum documento encontrado na pasta do projeto.",
        es: "No se encontró ningún documento en la carpeta del proyecto.",
        fr: "Aucun document trouvé dans le dossier du projet.",
        de: "Keine Dokumente im Projektordner gefunden.",
        it: "Nessun documento trovato nella cartella del progetto.",
        ja: "プロジェクトフォルダーにドキュメントが見つかりません。",
        zh: "在项目文件夹中没有找到文档。",
        ko: "프로젝트 폴더에서 문서를 찾지 못했어요.",
        ru: "В папке проекта документы не найдены." }
    /// Toast after saving the context mode failed.
    save_mode_failed { en: "Could not save the context mode.",
        pt: "Não foi possível salvar o modo de contexto.",
        es: "No se pudo guardar el modo de contexto.",
        fr: "Impossible d’enregistrer le mode de contexte.",
        de: "Der Kontextmodus konnte nicht gespeichert werden.",
        it: "Impossibile salvare la modalità di contesto.",
        ja: "コンテキストモードを保存できませんでした。",
        zh: "无法保存上下文模式。", ko: "컨텍스트 모드를 저장하지 못했어요.",
        ru: "Не удалось сохранить режим контекста." }
    /// Toast after adding a rule.
    rule_added { en: "Rule added.", pt: "Regra adicionada.", es: "Regla añadida.",
        fr: "Règle ajoutée.", de: "Regel hinzugefügt.", it: "Regola aggiunta.",
        ja: "ルールを追加しました。", zh: "已添加规则。", ko: "규칙을 추가했어요.",
        ru: "Правило добавлено." }
    /// Toast after ending a rule.
    rule_ended { en: "Rule ended. It stays in the history.",
        pt: "Regra encerrada. Ela continua no histórico.",
        es: "Regla cerrada. Sigue en el historial.",
        fr: "Règle clôturée. Elle reste dans l’historique.",
        de: "Regel beendet. Sie bleibt im Verlauf.",
        it: "Regola chiusa. Resta nella cronologia.",
        ja: "ルールを終了しました。履歴には残ります。",
        zh: "规则已终止，仍保留在历史中。",
        ko: "규칙을 종료했어요. 기록에는 남아 있어요.",
        ru: "Правило завершено. Оно остаётся в истории." }
    /// Error when the preview could not be built.
    pack_failed { en: "Could not build the preview. Describe the task in other words.",
        pt: "Não foi possível montar a prévia. Descreva a tarefa com outras palavras.",
        es: "No se pudo montar la vista previa. Describe la tarea con otras palabras.",
        fr: "Impossible de construire l’aperçu. Décrivez la tâche autrement.",
        de: "Die Vorschau konnte nicht erstellt werden. Beschreibe die Aufgabe mit anderen Worten.",
        it: "Impossibile creare l’anteprima. Descrivi l’attività con altre parole.",
        ja: "プレビューを作成できませんでした。タスクを別の言葉で説明してください。",
        zh: "无法生成预览。请换种说法描述任务。",
        ko: "미리 보기를 만들지 못했어요. 작업을 다른 말로 설명해 주세요.",
        ru: "Не удалось собрать предпросмотр. Опишите задачу другими словами." }
    /// Error when the export document could not be generated.
    export_render_failed { en: "Could not generate the file.",
        pt: "Não foi possível gerar o arquivo.",
        es: "No se pudo generar el archivo.",
        fr: "Impossible de générer le fichier.",
        de: "Die Datei konnte nicht erzeugt werden.",
        it: "Impossibile generare il file.",
        ja: "ファイルを生成できませんでした。", zh: "无法生成文件。",
        ko: "파일을 만들지 못했어요.", ru: "Не удалось создать файл." }
    /// Error when the export file could not be written.
    export_save_failed { en: "Could not save the file to that location.",
        pt: "Não foi possível salvar o arquivo nesse destino.",
        es: "No se pudo guardar el archivo en ese destino.",
        fr: "Impossible d’enregistrer le fichier à cet emplacement.",
        de: "Die Datei konnte an diesem Ort nicht gespeichert werden.",
        it: "Impossibile salvare il file in quella destinazione.",
        ja: "その場所にファイルを保存できませんでした。",
        zh: "无法将文件保存到该位置。", ko: "그 위치에 파일을 저장하지 못했어요.",
        ru: "Не удалось сохранить файл в это место." }
    /// Title of the export file dialog.
    export_dialog_title { en: "Export context preview",
        pt: "Exportar prévia de contexto", es: "Exportar vista previa de contexto",
        fr: "Exporter l’aperçu du contexte", de: "Kontextvorschau exportieren",
        it: "Esporta anteprima del contesto", ja: "コンテキストのプレビューをエクスポート",
        zh: "导出上下文预览", ko: "컨텍스트 미리 보기 내보내기",
        ru: "Экспорт предпросмотра контекста" }
    /// Filter name of the export file dialog.
    export_filter { en: "Document", pt: "Documento", es: "Documento", fr: "Document",
        de: "Dokument", it: "Documento", ja: "ドキュメント", zh: "文档", ko: "문서",
        ru: "Документ" }
    /// Default file name (without extension) of the exported preview.
    export_file_stem { en: "context", pt: "contexto", es: "contexto", fr: "contexte",
        de: "kontext", it: "contesto", ja: "context", zh: "context", ko: "context",
        ru: "context" }
    /// Error when the mode could not be read.
    read_mode_failed { en: "Could not read the context mode.",
        pt: "Não foi possível ler o modo de contexto.",
        es: "No se pudo leer el modo de contexto.",
        fr: "Impossible de lire le mode de contexte.",
        de: "Der Kontextmodus konnte nicht gelesen werden.",
        it: "Impossibile leggere la modalità di contesto.",
        ja: "コンテキストモードを読み込めませんでした。", zh: "无法读取上下文模式。",
        ko: "컨텍스트 모드를 읽지 못했어요.", ru: "Не удалось прочитать режим контекста." }
    /// Error when the decisions in force could not be read.
    read_decisions_failed { en: "Could not read the decisions in effect.",
        pt: "Não foi possível ler as decisões em vigor.",
        es: "No se pudieron leer las decisiones vigentes.",
        fr: "Impossible de lire les décisions en vigueur.",
        de: "Die Entscheidungen in Kraft konnten nicht gelesen werden.",
        it: "Impossibile leggere le decisioni in vigore.",
        ja: "有効な決定を読み込めませんでした。", zh: "无法读取生效中的决策。",
        ko: "유효한 결정을 읽지 못했어요.", ru: "Не удалось прочитать действующие решения." }
    /// Error when the deliveries could not be read.
    read_deliveries_failed { en: "Could not read the deliveries to the agent.",
        pt: "Não foi possível ler as entregas ao agente.",
        es: "No se pudieron leer las entregas al agente.",
        fr: "Impossible de lire les livraisons à l’agent.",
        de: "Die Lieferungen an den Agent konnten nicht gelesen werden.",
        it: "Impossibile leggere le consegne all’agente.",
        ja: "エージェントへの配信を読み込めませんでした。",
        zh: "无法读取交付给智能体的记录。", ko: "에이전트 전달 내역을 읽지 못했어요.",
        ru: "Не удалось прочитать доставки агенту." }
    /// Error when the project rules could not be read.
    read_rules_failed { en: "Could not read the project rules.",
        pt: "Não foi possível ler as regras do projeto.",
        es: "No se pudieron leer las reglas del proyecto.",
        fr: "Impossible de lire les règles du projet.",
        de: "Die Projektregeln konnten nicht gelesen werden.",
        it: "Impossibile leggere le regole del progetto.",
        ja: "プロジェクトのルールを読み込めませんでした。",
        zh: "无法读取项目规则。", ko: "프로젝트 규칙을 읽지 못했어요.",
        ru: "Не удалось прочитать правила проекта." }
    /// Error when the project documents could not be read.
    read_documents_failed { en: "Could not read the project documentation.",
        pt: "Não foi possível ler a documentação do projeto.",
        es: "No se pudo leer la documentación del proyecto.",
        fr: "Impossible de lire la documentation du projet.",
        de: "Die Projektdokumentation konnte nicht gelesen werden.",
        it: "Impossibile leggere la documentazione del progetto.",
        ja: "プロジェクトのドキュメントを読み込めませんでした。",
        zh: "无法读取项目文档。", ko: "프로젝트 문서를 읽지 못했어요.",
        ru: "Не удалось прочитать документацию проекта." }
    /// Error when the project folder is unreachable while indexing.
    folder_unavailable { en: "The project folder is not accessible; the last reading still applies.",
        pt: "A pasta do projeto não está acessível; a última leitura continua valendo.",
        es: "La carpeta del proyecto no está accesible; la última lectura sigue vigente.",
        fr: "Le dossier du projet n’est pas accessible ; la dernière lecture reste valable.",
        de: "Der Projektordner ist nicht erreichbar; die letzte Lesung gilt weiter.",
        it: "La cartella del progetto non è accessibile; l’ultima lettura resta valida.",
        ja: "プロジェクトフォルダーにアクセスできません。前回の読み込みが引き続き有効です。",
        zh: "无法访问项目文件夹，上次读取的结果仍然有效。",
        ko: "프로젝트 폴더에 접근할 수 없어요. 마지막으로 읽은 내용이 계속 유효해요.",
        ru: "Папка проекта недоступна; последнее чтение остаётся в силе." }
    /// Error when the project no longer exists while indexing.
    project_not_found { en: "Project not found.", pt: "Projeto não encontrado.",
        es: "Proyecto no encontrado.", fr: "Projet introuvable.",
        de: "Projekt nicht gefunden.", it: "Progetto non trovato.",
        ja: "プロジェクトが見つかりません。", zh: "未找到项目。",
        ko: "프로젝트를 찾을 수 없어요.", ru: "Проект не найден." }
    /// Error when indexing the documents failed in storage.
    index_documents_failed { en: "Could not read the documentation. Try again.",
        pt: "Não foi possível ler a documentação. Tente de novo.",
        es: "No se pudo leer la documentación. Inténtalo de nuevo.",
        fr: "Impossible de lire la documentation. Réessayez.",
        de: "Die Dokumentation konnte nicht gelesen werden. Versuche es erneut.",
        it: "Impossibile leggere la documentazione. Riprova.",
        ja: "ドキュメントを読み込めませんでした。もう一度お試しください。",
        zh: "无法读取文档，请重试。", ko: "문서를 읽지 못했어요. 다시 시도해 주세요.",
        ru: "Не удалось прочитать документацию. Повторите попытку." }
    /// Error when a new rule has no text.
    rule_empty { en: "Write the rule before adding it.",
        pt: "Escreva a regra antes de adicionar.",
        es: "Escribe la regla antes de añadirla.",
        fr: "Écrivez la règle avant de l’ajouter.",
        de: "Schreibe die Regel, bevor du sie hinzufügst.",
        it: "Scrivi la regola prima di aggiungerla.",
        ja: "追加する前にルールを書いてください。",
        zh: "请先写下规则再添加。", ko: "규칙을 먼저 작성한 뒤에 추가해 주세요.",
        ru: "Напишите правило, прежде чем добавлять его." }
    /// Error when a new rule is too long.
    rule_too_long { en: "The rule is too long; summarize it in one sentence.",
        pt: "A regra ficou longa demais; resuma em uma frase.",
        es: "La regla es demasiado larga; resúmela en una frase.",
        fr: "La règle est trop longue ; résumez-la en une phrase.",
        de: "Die Regel ist zu lang; fasse sie in einem Satz zusammen.",
        it: "La regola è troppo lunga; riassumila in una frase.",
        ja: "ルールが長すぎます。1文に要約してください。",
        zh: "规则太长了，请用一句话概括。", ko: "규칙이 너무 길어요. 한 문장으로 줄여 주세요.",
        ru: "Правило слишком длинное; сократите его до одного предложения." }
    /// Error when a rule changed while it was being edited.
    rule_changed { en: "The rule changed while you were editing. The list was updated.",
        pt: "A regra mudou enquanto você editava. A lista foi atualizada.",
        es: "La regla cambió mientras la editabas. La lista se actualizó.",
        fr: "La règle a changé pendant votre modification. La liste a été mise à jour.",
        de: "Die Regel hat sich geändert, während du sie bearbeitet hast. Die Liste wurde aktualisiert.",
        it: "La regola è cambiata mentre la modificavi. L’elenco è stato aggiornato.",
        ja: "編集中にルールが変更されました。一覧を更新しました。",
        zh: "你编辑时规则已发生变化，列表已更新。",
        ko: "편집하는 사이에 규칙이 바뀌었어요. 목록을 새로 고쳤어요.",
        ru: "Правило изменилось, пока вы его редактировали. Список обновлён." }
    /// Error when a rule could not be saved.
    rule_save_failed { en: "Could not save the rule.", pt: "Não foi possível salvar a regra.",
        es: "No se pudo guardar la regla.", fr: "Impossible d’enregistrer la règle.",
        de: "Die Regel konnte nicht gespeichert werden.",
        it: "Impossibile salvare la regola.", ja: "ルールを保存できませんでした。",
        zh: "无法保存规则。", ko: "규칙을 저장하지 못했어요.",
        ru: "Не удалось сохранить правило." }

    // Delivery mode page.

    /// Mode name: nothing is computed.
    mode_off { en: "Off", pt: "Desligado", es: "Desactivado", fr: "Désactivé",
        de: "Aus", it: "Disattivato", ja: "オフ", zh: "关闭", ko: "끔", ru: "Выключено" }
    /// Mode option: measure without sending.
    mode_measure { en: "Measure", pt: "Medir", es: "Medir", fr: "Mesurer",
        de: "Messen", it: "Misura", ja: "計測", zh: "测量", ko: "측정", ru: "Измерять" }
    /// Mode name: context is sent to the agent.
    mode_active { en: "Active", pt: "Ativo", es: "Activo", fr: "Actif", de: "Aktiv",
        it: "Attivo", ja: "オン", zh: "启用", ko: "켬", ru: "Включено" }
    /// Mode name in the status line: measuring without sending.
    mode_measuring { en: "Measuring", pt: "Medindo", es: "Midiendo", fr: "Mesure en cours",
        de: "Misst", it: "In misurazione", ja: "計測中", zh: "测量中", ko: "측정 중",
        ru: "Измеряется" }
    /// Description of the off mode.
    mode_off_body { en: "Nothing is computed or sent to the agent.",
        pt: "Nada é calculado nem enviado ao agente.",
        es: "No se calcula ni se envía nada al agente.",
        fr: "Rien n’est calculé ni envoyé à l’agent.",
        de: "Es wird nichts berechnet oder an den Agent gesendet.",
        it: "Non viene calcolato né inviato nulla all’agente.",
        ja: "計算も、エージェントへの送信も行いません。",
        zh: "不计算，也不发送给智能体。", ko: "계산도 하지 않고 에이전트에 보내지도 않아요.",
        ru: "Ничего не вычисляется и не отправляется агенту." }
    /// Description of the measure mode.
    mode_measure_body {
        en: "Computes the block and records how much would be sent, without sending.",
        pt: "Calcula o bloco e registra quanto seria enviado, sem enviar.",
        es: "Calcula el bloque y registra cuánto se enviaría, sin enviarlo.",
        fr: "Calcule le bloc et enregistre ce qui serait envoyé, sans l’envoyer.",
        de: "Berechnet den Block und protokolliert, wie viel gesendet würde, ohne zu senden.",
        it: "Calcola il blocco e registra quanto verrebbe inviato, senza inviarlo.",
        ja: "ブロックを計算し、送信される量を記録しますが、送信はしません。",
        zh: "计算内容块并记录将发送的量，但不发送。",
        ko: "블록을 계산하고 보냈을 분량을 기록하지만, 실제로 보내지는 않아요.",
        ru: "Вычисляет блок и записывает, сколько было бы отправлено, но не отправляет." }
    /// Description of the active mode.
    mode_active_body {
        en: "Attaches a compact block of decisions and rules to the agent's request.",
        pt: "Anexa um bloco compacto de decisões e regras ao pedido do agente.",
        es: "Adjunta un bloque compacto de decisiones y reglas a la solicitud del agente.",
        fr: "Joint un bloc compact de décisions et de règles à la requête de l’agent.",
        de: "Hängt einen kompakten Block aus Entscheidungen und Regeln an die Anfrage des Agents an.",
        it: "Allega un blocco compatto di decisioni e regole alla richiesta dell’agente.",
        ja: "決定とルールをまとめたコンパクトなブロックを、エージェントのリクエストに添付します。",
        zh: "把精简的决策和规则内容块附加到智能体的请求中。",
        ko: "결정과 규칙을 간추린 블록을 에이전트의 요청에 붙여요.",
        ru: "Добавляет к запросу агента компактный блок решений и правил." }
    /// Label of the mode radio list.
    mode_aria { en: "Context mode", pt: "Modo de contexto", es: "Modo de contexto",
        fr: "Mode de contexte", de: "Kontextmodus", it: "Modalità di contesto",
        ja: "コンテキストモード", zh: "上下文模式", ko: "컨텍스트 모드",
        ru: "Режим контекста" }
    /// Label of the budget field and figure: tokens in each block.
    tokens_per_block { en: "Tokens per block", pt: "Tokens por bloco",
        es: "Tokens por bloque", fr: "Tokens par bloc", de: "Tokens pro Block",
        it: "Token per blocco", ja: "ブロックあたりのトークン数", zh: "每块令牌数",
        ko: "블록당 토큰 수", ru: "Токенов на блок" }
    /// Label of the button that saves the mode.
    save_mode_aria { en: "Save context mode", pt: "Salvar modo de contexto",
        es: "Guardar modo de contexto", fr: "Enregistrer le mode de contexte",
        de: "Kontextmodus speichern", it: "Salva modalità di contesto",
        ja: "コンテキストモードを保存", zh: "保存上下文模式", ko: "컨텍스트 모드 저장",
        ru: "Сохранить режим контекста" }
    /// Save button.
    save { en: "Save", pt: "Salvar", es: "Guardar", fr: "Enregistrer", de: "Speichern",
        it: "Salva", ja: "保存", zh: "保存", ko: "저장", ru: "Сохранить" }

    // Decisions page.

    /// Empty state of the decisions in force.
    no_decisions { en: "No confirmed decisions yet. Confirm candidates in Review.",
        pt: "Nenhuma decisão confirmada ainda. Confirme candidatos em Revisão.",
        es: "Aún no hay decisiones confirmadas. Confirma candidatos en Revisión.",
        fr: "Aucune décision confirmée pour l’instant. Confirmez des candidats dans Revue.",
        de: "Noch keine bestätigten Entscheidungen. Bestätige Kandidaten unter Prüfung.",
        it: "Ancora nessuna decisione confermata. Conferma i candidati in Revisione.",
        ja: "確定した決定はまだありません。レビューで候補を確定してください。",
        zh: "还没有已确认的决策。请在审阅中确认候选。",
        ko: "아직 확정된 결정이 없어요. 검토에서 후보를 확인해 주세요.",
        ru: "Подтверждённых решений пока нет. Подтвердите кандидатов в разделе «Проверка»." }

    // Documents page.

    /// Label of the button that rereads the documentation.
    reread_docs_aria { en: "Read the documentation again",
        pt: "Ler a documentação de novo", es: "Volver a leer la documentación",
        fr: "Relire la documentation", de: "Dokumentation erneut lesen",
        it: "Rileggi la documentazione", ja: "ドキュメントを再読み込み",
        zh: "重新读取文档", ko: "문서 다시 읽기", ru: "Прочитать документацию заново" }
    /// Button that rereads the documentation.
    reread { en: "Read again", pt: "Ler de novo", es: "Leer de nuevo", fr: "Relire",
        de: "Erneut lesen", it: "Rileggi", ja: "再読み込み", zh: "重新读取",
        ko: "다시 읽기", ru: "Прочитать заново" }
    /// Header of the documents page when nothing was read.
    no_documents_read { en: "No documents read yet.", pt: "Nenhum documento lido ainda.",
        es: "Aún no se ha leído ningún documento.", fr: "Aucun document lu pour l’instant.",
        de: "Noch keine Dokumente gelesen.", it: "Ancora nessun documento letto.",
        ja: "まだドキュメントを読み込んでいません。", zh: "还没有读取任何文档。",
        ko: "아직 읽은 문서가 없어요.", ru: "Документы ещё не прочитаны." }
    /// Empty state of the documents page.
    documents_empty { en: "No documents found. Markdown files in docs/, specs/, adr/ and the root README show up here.",
        pt: "Nenhum documento encontrado. Arquivos Markdown em docs/, specs/, adr/ e o README da raiz aparecem aqui.",
        es: "No se encontró ningún documento. Los archivos Markdown de docs/, specs/, adr/ y el README de la raíz aparecen aquí.",
        fr: "Aucun document trouvé. Les fichiers Markdown de docs/, specs/, adr/ et le README de la racine apparaissent ici.",
        de: "Keine Dokumente gefunden. Markdown-Dateien in docs/, specs/, adr/ und die README im Stammordner erscheinen hier.",
        it: "Nessun documento trovato. I file Markdown in docs/, specs/, adr/ e il README della radice compaiono qui.",
        ja: "ドキュメントが見つかりません。docs/、specs/、adr/ の Markdown ファイルとルートの README がここに表示されます。",
        zh: "没有找到文档。docs/、specs/、adr/ 中的 Markdown 文件和根目录的 README 会显示在这里。",
        ko: "문서를 찾지 못했어요. docs/, specs/, adr/ 안의 Markdown 파일과 루트의 README가 여기에 나타나요.",
        ru: "Документы не найдены. Здесь появятся Markdown-файлы из docs/, specs/, adr/ и README в корне." }
    /// Group heading: decision records.
    kind_adr { en: "Decision records (ADR)", pt: "Registros de decisão (ADR)",
        es: "Registros de decisión (ADR)", fr: "Journaux de décisions (ADR)",
        de: "Entscheidungsprotokolle (ADR)", it: "Registri delle decisioni (ADR)",
        ja: "決定記録 (ADR)", zh: "决策记录 (ADR)", ko: "결정 기록 (ADR)",
        ru: "Записи о решениях (ADR)" }
    /// Group heading: specifications.
    kind_spec { en: "Specifications", pt: "Especificações", es: "Especificaciones",
        fr: "Spécifications", de: "Spezifikationen", it: "Specifiche", ja: "仕様書",
        zh: "规格说明", ko: "명세서", ru: "Спецификации" }
    /// Group heading: README files.
    kind_readme { en: "READMEs", pt: "READMEs", es: "READMEs", fr: "READMEs",
        de: "READMEs", it: "README", ja: "README", zh: "README", ko: "README",
        ru: "README" }
    /// Group heading: guides and notes.
    kind_guide { en: "Guides and notes", pt: "Guias e notas", es: "Guías y notas",
        fr: "Guides et notes", de: "Anleitungen und Notizen", it: "Guide e note",
        ja: "ガイドとメモ", zh: "指南和笔记", ko: "가이드와 노트", ru: "Руководства и заметки" }

    // Rules page.

    /// Empty state of the rules page.
    rules_empty { en: "No rules yet. Assumptions, constraints, goals and conventions apply to every task in the project and go into the agent's context.",
        pt: "Sem regras ainda. Premissas, restrições, objetivos e convenções valem para todas as tarefas do projeto e entram no contexto do agente.",
        es: "Aún no hay reglas. Los supuestos, las restricciones, los objetivos y las convenciones valen para todas las tareas del proyecto y entran en el contexto del agente.",
        fr: "Pas encore de règles. Les hypothèses, contraintes, objectifs et conventions valent pour toutes les tâches du projet et entrent dans le contexte de l’agent.",
        de: "Noch keine Regeln. Annahmen, Einschränkungen, Ziele und Konventionen gelten für alle Aufgaben des Projekts und fließen in den Kontext des Agents ein.",
        it: "Ancora nessuna regola. Presupposti, vincoli, obiettivi e convenzioni valgono per tutte le attività del progetto ed entrano nel contesto dell’agente.",
        ja: "ルールはまだありません。前提・制約・目標・規約はプロジェクトのすべてのタスクに適用され、エージェントのコンテキストに入ります。",
        zh: "还没有规则。前提、约束、目标和约定适用于项目的所有任务，并会进入智能体的上下文。",
        ko: "아직 규칙이 없어요. 전제, 제약, 목표, 컨벤션은 프로젝트의 모든 작업에 적용되고 에이전트의 컨텍스트에 들어가요.",
        ru: "Правил пока нет. Допущения, ограничения, цели и соглашения действуют для всех задач проекта и попадают в контекст агента." }
    /// Label of the rule kind radio group.
    rule_kind_aria { en: "Rule type", pt: "Tipo da regra", es: "Tipo de regla",
        fr: "Type de règle", de: "Regeltyp", it: "Tipo di regola", ja: "ルールの種類",
        zh: "规则类型", ko: "규칙 유형", ru: "Тип правила" }
    /// Label of the button that adds a rule.
    add_rule_aria { en: "Add rule", pt: "Adicionar regra", es: "Añadir regla",
        fr: "Ajouter la règle", de: "Regel hinzufügen", it: "Aggiungi regola",
        ja: "ルールを追加", zh: "添加规则", ko: "규칙 추가", ru: "Добавить правило" }
    /// Button that adds a rule.
    add { en: "Add", pt: "Adicionar", es: "Añadir", fr: "Ajouter", de: "Hinzufügen",
        it: "Aggiungi", ja: "追加", zh: "添加", ko: "추가", ru: "Добавить" }
    /// Kind of rule: assumption.
    kind_assumption { en: "Assumption", pt: "Premissa", es: "Supuesto", fr: "Hypothèse",
        de: "Annahme", it: "Presupposto", ja: "前提", zh: "前提", ko: "전제",
        ru: "Допущение" }
    /// Kind of rule: constraint.
    kind_constraint { en: "Constraint", pt: "Restrição", es: "Restricción",
        fr: "Contrainte", de: "Einschränkung", it: "Vincolo", ja: "制約", zh: "约束",
        ko: "제약", ru: "Ограничение" }
    /// Kind of rule: goal.
    kind_goal { en: "Goal", pt: "Objetivo", es: "Objetivo", fr: "Objectif", de: "Ziel",
        it: "Obiettivo", ja: "目標", zh: "目标", ko: "목표", ru: "Цель" }
    /// Kind of rule: convention.
    kind_convention { en: "Convention", pt: "Convenção", es: "Convención",
        fr: "Convention", de: "Konvention", it: "Convenzione", ja: "規約", zh: "约定",
        ko: "컨벤션", ru: "Соглашение" }
    /// Group heading: assumptions.
    kinds_assumption { en: "Assumptions", pt: "Premissas", es: "Supuestos",
        fr: "Hypothèses", de: "Annahmen", it: "Presupposti", ja: "前提", zh: "前提",
        ko: "전제", ru: "Допущения" }
    /// Group heading: constraints.
    kinds_constraint { en: "Constraints", pt: "Restrições", es: "Restricciones",
        fr: "Contraintes", de: "Einschränkungen", it: "Vincoli", ja: "制約", zh: "约束",
        ko: "제약", ru: "Ограничения" }
    /// Group heading: goals.
    kinds_goal { en: "Goals", pt: "Objetivos", es: "Objetivos", fr: "Objectifs",
        de: "Ziele", it: "Obiettivi", ja: "目標", zh: "目标", ko: "목표", ru: "Цели" }
    /// Group heading: conventions.
    kinds_convention { en: "Conventions", pt: "Convenções", es: "Convenciones",
        fr: "Conventions", de: "Konventionen", it: "Convenzioni", ja: "規約", zh: "约定",
        ko: "컨벤션", ru: "Соглашения" }
    /// Fallback kind name of a rule with an unknown kind.
    rule_word { en: "Rule", pt: "Regra", es: "Regla", fr: "Règle", de: "Regel",
        it: "Regola", ja: "ルール", zh: "规则", ko: "규칙", ru: "Правило" }
    /// Line under a rule without inherited scope.
    inherited_scope_missing { en: "Inherited scope not provided.",
        pt: "Escopo herdado não informado.", es: "Alcance heredado no indicado.",
        fr: "Portée héritée non renseignée.", de: "Geerbter Geltungsbereich nicht angegeben.",
        it: "Ambito ereditato non indicato.", ja: "継承された範囲は指定されていません。",
        zh: "未提供继承的范围。", ko: "상속된 범위가 지정되지 않았어요.",
        ru: "Унаследованная область не указана." }
    /// Error when the qualifiers or scope of a rule are unreadable.
    qualifiers_unreadable { en: "Could not read this rule's qualifiers or scope.",
        pt: "Não foi possível ler os qualificadores ou o escopo desta regra.",
        es: "No se pudieron leer los calificadores ni el alcance de esta regla.",
        fr: "Impossible de lire les qualificatifs ou la portée de cette règle.",
        de: "Die Qualifikatoren oder der Geltungsbereich dieser Regel konnten nicht gelesen werden.",
        it: "Impossibile leggere i qualificatori o l’ambito di questa regola.",
        ja: "このルールの修飾子または範囲を読み込めませんでした。",
        zh: "无法读取此规则的限定条件或范围。",
        ko: "이 규칙의 한정자나 범위를 읽지 못했어요.",
        ru: "Не удалось прочитать квалификаторы или область этого правила." }
    /// Label of the button that rereads qualifiers and scope.
    retry_qualifiers_aria { en: "Try reading the qualifiers and scope again",
        pt: "Tentar ler qualificadores e escopo novamente",
        es: "Intentar leer de nuevo los calificadores y el alcance",
        fr: "Réessayer de lire les qualificatifs et la portée",
        de: "Qualifikatoren und Geltungsbereich erneut lesen",
        it: "Riprova a leggere qualificatori e ambito",
        ja: "修飾子と範囲をもう一度読み込む", zh: "重新读取限定条件和范围",
        ko: "한정자와 범위를 다시 읽기", ru: "Повторить чтение квалификаторов и области" }
    /// Retry button next to a rule error.
    try_again_long { en: "Try again", pt: "Tentar novamente", es: "Reintentar",
        fr: "Réessayer", de: "Erneut versuchen", it: "Riprova", ja: "再試行",
        zh: "重试", ko: "다시 시도", ru: "Повторить" }
    /// Pill on a rule whose source decision was superseded.
    review_pill { en: "Review", pt: "Revisar", es: "Revisar", fr: "Revoir",
        de: "Prüfen", it: "Rivedi", ja: "要確認", zh: "待审阅", ko: "검토 필요",
        ru: "Проверить" }
    /// Question shown while confirming the end of a rule.
    retire_confirm { en: "End today? It stops applying and stays in the history.",
        pt: "Encerrar hoje? Ela deixa de valer e fica no histórico.",
        es: "¿Cerrar hoy? Deja de valer y queda en el historial.",
        fr: "Clôturer aujourd’hui ? Elle cesse de s’appliquer et reste dans l’historique.",
        de: "Heute beenden? Sie gilt nicht mehr und bleibt im Verlauf.",
        it: "Chiudere oggi? Smette di valere e resta nella cronologia.",
        ja: "今日で終了しますか？ 適用されなくなり、履歴には残ります。",
        zh: "今天终止吗？规则将不再生效，并保留在历史中。",
        ko: "오늘 종료할까요? 더는 적용되지 않고 기록에는 남아요.",
        ru: "Завершить сегодня? Правило перестанет действовать и останется в истории." }
    /// Hint on a rule whose source decision was superseded.
    source_superseded { en: "The source decision was superseded; check whether it still applies.",
        pt: "A decisão de origem foi substituída; confira se ainda vale.",
        es: "La decisión de origen fue sustituida; comprueba si sigue vigente.",
        fr: "La décision d’origine a été remplacée ; vérifiez si elle s’applique toujours.",
        de: "Die ursprüngliche Entscheidung wurde ersetzt; prüfe, ob die Regel noch gilt.",
        it: "La decisione di origine è stata sostituita; verifica se vale ancora.",
        ja: "元の決定が置き換えられました。今も有効か確認してください。",
        zh: "来源决策已被取代，请确认规则是否仍然适用。",
        ko: "원본 결정이 대체됐어요. 아직 유효한지 확인해 주세요.",
        ru: "Исходное решение заменено; проверьте, действует ли правило до сих пор." }
    /// Cancel button.
    cancel { en: "Cancel", pt: "Cancelar", es: "Cancelar", fr: "Annuler",
        de: "Abbrechen", it: "Annulla", ja: "キャンセル", zh: "取消", ko: "취소",
        ru: "Отмена" }
    /// Label and text of the button that ends a rule.
    retire { en: "End", pt: "Encerrar", es: "Cerrar", fr: "Clôturer", de: "Beenden",
        it: "Chiudi", ja: "終了", zh: "终止", ko: "종료", ru: "Завершить" }
    /// Label of the button that ends a rule.
    retire_rule_aria { en: "End rule", pt: "Encerrar regra", es: "Cerrar regla",
        fr: "Clôturer la règle", de: "Regel beenden", it: "Chiudi regola",
        ja: "ルールを終了", zh: "终止规则", ko: "규칙 종료", ru: "Завершить правило" }

    // Preview page.

    /// Shown when nothing in the project matched the typed task.
    pack_no_match { en: "Nothing in this project matched the task. The agent would not get any context for it.",
        pt: "Nada deste projeto casou com a tarefa. O agente não receberia contexto para ela.",
        es: "Nada de este proyecto coincidió con la tarea. El agente no recibiría contexto para ella.",
        fr: "Rien dans ce projet ne correspond à la tâche. L’agent ne recevrait aucun contexte pour elle.",
        de: "Nichts aus diesem Projekt passte zur Aufgabe. Der Agent bekäme dafür keinen Kontext.",
        it: "Nulla di questo progetto corrisponde all’attività. L’agente non riceverebbe alcun contesto per essa.",
        ja: "このプロジェクトにはタスクに一致するものがありません。エージェントはこのタスクのコンテキストを受け取りません。",
        zh: "此项目中没有与该任务匹配的内容。智能体不会收到针对它的上下文。",
        ko: "이 프로젝트에서 작업과 맞는 내용이 없어요. 에이전트는 이 작업에 대한 컨텍스트를 받지 못해요.",
        ru: "В этом проекте ничего не подошло под задачу. Агент не получил бы для неё контекста." }
    /// Line under a decision of the preview without scope.
    scope_missing { en: "Scope not provided.", pt: "Escopo não informado.",
        es: "Alcance no indicado.", fr: "Portée non renseignée.",
        de: "Geltungsbereich nicht angegeben.", it: "Ambito non indicato.",
        ja: "範囲は指定されていません。", zh: "未提供范围。", ko: "범위가 지정되지 않았어요.",
        ru: "Область не указана." }
    /// Label of the button that exports the preview as Markdown.
    export_markdown_aria { en: "Export as Markdown", pt: "Exportar em Markdown",
        es: "Exportar como Markdown", fr: "Exporter en Markdown",
        de: "Als Markdown exportieren", it: "Esporta in Markdown",
        ja: "Markdown でエクスポート", zh: "导出为 Markdown", ko: "Markdown으로 내보내기",
        ru: "Экспортировать в Markdown" }
    /// Label of the button that exports the preview as JSON.
    export_json_aria { en: "Export as JSON", pt: "Exportar em JSON",
        es: "Exportar como JSON", fr: "Exporter en JSON", de: "Als JSON exportieren",
        it: "Esporta in JSON", ja: "JSON でエクスポート", zh: "导出为 JSON",
        ko: "JSON으로 내보내기", ru: "Экспортировать в JSON" }
    /// Button and label that build the preview.
    build_preview { en: "Build preview", pt: "Montar prévia", es: "Montar vista previa",
        fr: "Construire l’aperçu", de: "Vorschau erstellen", it: "Crea anteprima",
        ja: "プレビューを作成", zh: "生成预览", ko: "미리 보기 만들기",
        ru: "Собрать предпросмотр" }

    // Page titles and subtitles.

    /// Title of the overview page.
    overview_title { en: "Agent context", pt: "Contexto do agente",
        es: "Contexto del agente", fr: "Contexte de l’agent", de: "Kontext des Agents",
        it: "Contesto dell’agente", ja: "エージェントのコンテキスト",
        zh: "智能体上下文", ko: "에이전트 컨텍스트", ru: "Контекст агента" }
    /// Subtitle of the overview page.
    overview_subtitle { en: "What the coding agent receives from this project, where it comes from and how it gets there.",
        pt: "O que o agente de código recebe deste projeto, de onde vem e como chega a ele.",
        es: "Lo que el agente de código recibe de este proyecto, de dónde viene y cómo le llega.",
        fr: "Ce que l’agent de code reçoit de ce projet, d’où cela vient et comment cela lui parvient.",
        de: "Was der Code-Agent aus diesem Projekt erhält, woher es stammt und wie es ihn erreicht.",
        it: "Ciò che l’agente di codice riceve da questo progetto, da dove viene e come gli arriva.",
        ja: "コーディングエージェントがこのプロジェクトから受け取る内容と、その出どころ、届き方。",
        zh: "编码智能体从这个项目收到什么、来自哪里、如何送达。",
        ko: "코딩 에이전트가 이 프로젝트에서 무엇을 받는지, 어디서 오는지, 어떻게 전달되는지 보여줘요.",
        ru: "Что кодовый агент получает из этого проекта, откуда это берётся и как доходит до него." }
    /// Subtitle of the deliveries page.
    deliveries_subtitle { en: "Each block computed for the agent: whether it was sent or only measured, how much it took and what was left out of the budget.",
        pt: "Cada bloco calculado para o agente: se foi enviado ou só medido, quanto ocupou e o que ficou de fora do orçamento.",
        es: "Cada bloque calculado para el agente: si se envió o solo se midió, cuánto ocupó y qué quedó fuera del presupuesto.",
        fr: "Chaque bloc calculé pour l’agent : envoyé ou seulement mesuré, sa taille et ce qui est resté hors budget.",
        de: "Jeder für den Agent berechnete Block: ob er gesendet oder nur gemessen wurde, wie viel er belegte und was außerhalb des Budgets blieb.",
        it: "Ogni blocco calcolato per l’agente: se è stato inviato o solo misurato, quanto ha occupato e cosa è rimasto fuori dal budget.",
        ja: "エージェント向けに計算した各ブロック。送信か計測のみか、使用量、予算に収まらなかった項目を確認できます。",
        zh: "为智能体计算的每个内容块：已发送还是仅测量、占用多少，以及哪些超出了预算。",
        ko: "에이전트를 위해 계산한 각 블록이에요. 보냈는지 측정만 했는지, 분량은 얼마인지, 예산에서 빠진 것은 무엇인지 보여줘요.",
        ru: "Каждый блок, рассчитанный для агента: отправлен или только измерен, сколько занял и что не поместилось в бюджет." }
    /// Subtitle of the preview page.
    test_subtitle { en: "Describe a task the way you would ask the agent and see which decisions and rules would go into its context.",
        pt: "Descreva uma tarefa como pediria ao agente e veja quais decisões e regras entrariam no contexto dela.",
        es: "Describe una tarea como se la pedirías al agente y mira qué decisiones y reglas entrarían en su contexto.",
        fr: "Décrivez une tâche comme vous la demanderiez à l’agent et voyez quelles décisions et règles entreraient dans son contexte.",
        de: "Beschreibe eine Aufgabe so, wie du sie dem Agent stellen würdest, und sieh, welche Entscheidungen und Regeln in ihren Kontext kämen.",
        it: "Descrivi un’attività come la chiederesti all’agente e vedi quali decisioni e regole entrerebbero nel suo contesto.",
        ja: "エージェントに頼むときのようにタスクを書くと、そのコンテキストに入る決定とルールを確認できます。",
        zh: "像向智能体提需求那样描述一个任务，看看哪些决策和规则会进入它的上下文。",
        ko: "에이전트에게 요청하듯 작업을 적으면, 그 컨텍스트에 어떤 결정과 규칙이 들어가는지 볼 수 있어요.",
        ru: "Опишите задачу так, как попросили бы агента, и посмотрите, какие решения и правила попали бы в её контекст." }
    /// Title of the decisions page.
    decisions_title { en: "Decisions in effect", pt: "Decisões em vigor",
        es: "Decisiones vigentes", fr: "Décisions en vigueur", de: "Entscheidungen in Kraft",
        it: "Decisioni in vigore", ja: "有効な決定", zh: "生效中的决策",
        ko: "유효한 결정", ru: "Действующие решения" }
    /// Subtitle of the decisions page.
    decisions_subtitle { en: "Confirmed and not superseded: they go into the context when the task touches what they decide. Open one to see its history.",
        pt: "Confirmadas e não substituídas: entram no contexto quando a tarefa toca no que decidem. Abra uma para ver o histórico.",
        es: "Confirmadas y no sustituidas: entran en el contexto cuando la tarea toca lo que deciden. Abre una para ver su historial.",
        fr: "Confirmées et non remplacées : elles entrent dans le contexte quand la tâche touche à ce qu’elles décident. Ouvrez-en une pour voir son historique.",
        de: "Bestätigt und nicht ersetzt: Sie fließen in den Kontext ein, wenn die Aufgabe berührt, was sie entscheiden. Öffne eine, um ihren Verlauf zu sehen.",
        it: "Confermate e non sostituite: entrano nel contesto quando l’attività tocca ciò che decidono. Aprine una per vedere la cronologia.",
        ja: "確定済みで置き換えられていない決定です。タスクが決定の内容に関わるとコンテキストに入ります。開くと履歴を確認できます。",
        zh: "已确认且未被取代：当任务涉及它们所决定的内容时，会进入上下文。打开一项可查看其历史。",
        ko: "확정되었고 대체되지 않은 결정이에요. 작업이 결정 내용과 관련될 때 컨텍스트에 들어가요. 하나를 열면 기록을 볼 수 있어요.",
        ru: "Подтверждённые и не заменённые: попадают в контекст, когда задача затрагивает то, что они определяют. Откройте любое, чтобы увидеть историю." }
    /// Subtitle of the rules page.
    rules_subtitle { en: "Assumptions, constraints, goals and conventions that apply to every task in the project.",
        pt: "Premissas, restrições, objetivos e convenções que valem para todas as tarefas do projeto.",
        es: "Supuestos, restricciones, objetivos y convenciones que valen para todas las tareas del proyecto.",
        fr: "Hypothèses, contraintes, objectifs et conventions qui valent pour toutes les tâches du projet.",
        de: "Annahmen, Einschränkungen, Ziele und Konventionen, die für alle Aufgaben des Projekts gelten.",
        it: "Presupposti, vincoli, obiettivi e convenzioni che valgono per tutte le attività del progetto.",
        ja: "プロジェクトのすべてのタスクに適用される前提・制約・目標・規約です。",
        zh: "适用于项目所有任务的前提、约束、目标和约定。",
        ko: "프로젝트의 모든 작업에 적용되는 전제, 제약, 목표, 컨벤션이에요.",
        ru: "Допущения, ограничения, цели и соглашения, которые действуют для всех задач проекта." }
    /// Subtitle of the documentation page.
    documents_subtitle { en: "Captured and indexed from the project folder (docs/, specs/, ADRs and README). It feeds the Overview as a source and can generate candidates after analysis. Decisions and rules are only created after confirmation in Review; the document is not sent directly to the agent.",
        pt: "Capturada e indexada da pasta do projeto (docs/, specs/, ADRs e README). Alimenta a Visão como fonte e pode gerar candidatos após análise. Decisões e regras só são criadas após confirmação na Revisão; o documento não é enviado diretamente ao agente.",
        es: "Capturada e indexada desde la carpeta del proyecto (docs/, specs/, ADR y README). Alimenta la Visión general como fuente y puede generar candidatos tras el análisis. Las decisiones y reglas solo se crean tras confirmarlas en Revisión; el documento no se envía directamente al agente.",
        fr: "Capturée et indexée depuis le dossier du projet (docs/, specs/, ADR et README). Elle alimente la Vue d’ensemble comme source et peut générer des candidats après analyse. Les décisions et règles ne sont créées qu’après confirmation dans Revue ; le document n’est pas envoyé directement à l’agent.",
        de: "Aus dem Projektordner erfasst und indexiert (docs/, specs/, ADRs und README). Sie speist die Übersicht als Quelle und kann nach der Analyse Kandidaten erzeugen. Entscheidungen und Regeln entstehen erst nach Bestätigung unter Prüfung; das Dokument wird nicht direkt an den Agent gesendet.",
        it: "Acquisita e indicizzata dalla cartella del progetto (docs/, specs/, ADR e README). Alimenta la Panoramica come fonte e può generare candidati dopo l’analisi. Decisioni e regole vengono create solo dopo la conferma in Revisione; il documento non viene inviato direttamente all’agente.",
        ja: "プロジェクトフォルダー(docs/、specs/、ADR、README)から取り込み、インデックス化します。概要のソースになり、分析後に候補を生成することもあります。決定とルールはレビューで確定して初めて作られ、ドキュメント自体がエージェントに直接送られることはありません。",
        zh: "从项目文件夹（docs/、specs/、ADR 和 README）捕获并建立索引。它作为来源供给概览，分析后可生成候选。决策和规则只有在审阅中确认后才会创建；文档本身不会直接发送给智能体。",
        ko: "프로젝트 폴더(docs/, specs/, ADR, README)에서 수집해 색인해요. 개요의 출처가 되고, 분석 후 후보를 만들 수 있어요. 결정과 규칙은 검토에서 확인한 뒤에만 만들어지며, 문서가 에이전트에 직접 전달되지는 않아요.",
        ru: "Собрана и проиндексирована из папки проекта (docs/, specs/, ADR и README). Служит источником для обзора и после анализа может порождать кандидатов. Решения и правила создаются только после подтверждения в разделе «Проверка»; сам документ агенту напрямую не отправляется." }
    /// Subtitle of the delivery mode page.
    mode_subtitle { en: "When the agent receives context and how much fits in each block.",
        pt: "Quando o agente recebe contexto e quanto cabe em cada bloco.",
        es: "Cuándo recibe contexto el agente y cuánto cabe en cada bloque.",
        fr: "Quand l’agent reçoit du contexte et ce qui tient dans chaque bloc.",
        de: "Wann der Agent Kontext erhält und wie viel in jeden Block passt.",
        it: "Quando l’agente riceve il contesto e quanto entra in ogni blocco.",
        ja: "エージェントがいつコンテキストを受け取るか、各ブロックにどれだけ入るか。",
        zh: "智能体何时收到上下文，以及每个内容块能容纳多少。",
        ko: "에이전트가 언제 컨텍스트를 받는지, 블록마다 얼마나 담기는지 정해요.",
        ru: "Когда агент получает контекст и сколько помещается в каждый блок." }

    // Overview page.

    /// Label of the button that opens the delivery mode page.
    change_mode_aria { en: "Change the delivery mode", pt: "Mudar o modo de entrega",
        es: "Cambiar el modo de entrega", fr: "Changer le mode de livraison",
        de: "Lieferungsmodus ändern", it: "Cambia la modalità di consegna",
        ja: "配信モードを変更", zh: "更改交付模式", ko: "전달 모드 변경",
        ru: "Изменить режим доставки" }
    /// Button on the overview when the mode is off.
    activate { en: "Turn on", pt: "Ativar", es: "Activar", fr: "Activer",
        de: "Aktivieren", it: "Attiva", ja: "オンにする", zh: "启用", ko: "켜기",
        ru: "Включить" }
    /// Button on the overview when a mode is set.
    change_mode { en: "Change mode", pt: "Mudar modo", es: "Cambiar modo",
        fr: "Changer de mode", de: "Modus ändern", it: "Cambia modalità",
        ja: "モードを変更", zh: "更改模式", ko: "모드 변경", ru: "Изменить режим" }
    /// Pipeline stage II: what is selected.
    stage_selection { en: "Selection", pt: "Seleção", es: "Selección", fr: "Sélection",
        de: "Auswahl", it: "Selezione", ja: "選択", zh: "筛选", ko: "선택",
        ru: "Отбор" }
    /// Pipeline stage III: what is delivered.
    stage_delivery { en: "Delivery", pt: "Entrega", es: "Entrega", fr: "Livraison",
        de: "Lieferung", it: "Consegna", ja: "配信", zh: "交付", ko: "전달",
        ru: "Доставка" }
    /// Selection stage: how a request is matched.
    stage_on_request { en: "On request: the text and the files it names.",
        pt: "No pedido: o texto e os arquivos citados.",
        es: "En la solicitud: el texto y los archivos citados.",
        fr: "À la requête : le texte et les fichiers cités.",
        de: "Bei der Anfrage: der Text und die genannten Dateien.",
        it: "Nella richiesta: il testo e i file citati.",
        ja: "リクエスト時: 本文と、挙げられたファイル。",
        zh: "请求时：文本和其中提到的文件。",
        ko: "요청할 때: 텍스트와 언급된 파일.",
        ru: "В запросе: текст и упомянутые файлы." }
    /// Selection stage: how an edit is matched.
    stage_on_edit { en: "On edit: what the map links to the file.",
        pt: "Na edição: o que o mapa liga ao arquivo.",
        es: "Al editar: lo que el mapa vincula al archivo.",
        fr: "À la modification : ce que la carte relie au fichier.",
        de: "Beim Bearbeiten: was die Karte mit der Datei verknüpft.",
        it: "Alla modifica: ciò che la mappa collega al file.",
        ja: "編集時: マップがそのファイルに結び付けている内容。",
        zh: "编辑时：地图与该文件关联的内容。",
        ko: "편집할 때: 맵이 그 파일에 연결해 둔 내용.",
        ru: "При правке: то, что карта связывает с файлом." }
    /// Delivery stage: no repeats.
    stage_no_repeat { en: "Without repeating what the session already received.",
        pt: "Sem repetir o que a sessão já recebeu.",
        es: "Sin repetir lo que la sesión ya recibió.",
        fr: "Sans répéter ce que la session a déjà reçu.",
        de: "Ohne zu wiederholen, was die Sitzung schon erhalten hat.",
        it: "Senza ripetere ciò che la sessione ha già ricevuto.",
        ja: "セッションがすでに受け取った内容は繰り返しません。",
        zh: "不重复会话已经收到的内容。",
        ko: "세션이 이미 받은 내용은 반복하지 않아요.",
        ru: "Без повторения того, что сессия уже получила." }
    /// Section label of the pipeline on the overview.
    how_it_arrives { en: "How it reaches the agent", pt: "Como chega ao agente",
        es: "Cómo llega al agente", fr: "Comment cela parvient à l’agent",
        de: "Wie es den Agent erreicht", it: "Come arriva all’agente",
        ja: "エージェントへの届き方", zh: "如何送达智能体", ko: "에이전트에 전달되는 방법",
        ru: "Как это доходит до агента" }
    /// Section label of the weekly figures.
    last_seven_days { en: "Last 7 days", pt: "Últimos 7 dias", es: "Últimos 7 días",
        fr: "7 derniers jours", de: "Letzte 7 Tage", it: "Ultimi 7 giorni",
        ja: "過去7日間", zh: "最近 7 天", ko: "최근 7일", ru: "Последние 7 дней" }
    /// Section label of the latest deliveries.
    recent_deliveries { en: "Recent deliveries", pt: "Entregas recentes",
        es: "Entregas recientes", fr: "Livraisons récentes",
        de: "Letzte Lieferungen", it: "Consegne recenti", ja: "最近の配信",
        zh: "最近的交付", ko: "최근 전달", ru: "Недавние доставки" }
    /// Figure label: agent sessions.
    figure_sessions { en: "Agent sessions", pt: "Sessões do agente",
        es: "Sesiones del agente", fr: "Sessions de l’agent", de: "Agent-Sitzungen",
        it: "Sessioni dell’agente", ja: "エージェントのセッション", zh: "智能体会话",
        ko: "에이전트 세션", ru: "Сессии агента" }
    /// Figure label: items left out of the budget.
    figure_omitted { en: "Out of budget", pt: "Fora do orçamento", es: "Fuera del presupuesto",
        fr: "Hors budget", de: "Außerhalb des Budgets", it: "Fuori budget",
        ja: "予算外", zh: "超出预算", ko: "예산 초과", ru: "Вне бюджета" }
    /// Detail under the sessions figure.
    sessions_detail { en: "conversations that received", pt: "conversas que receberam",
        es: "conversaciones que recibieron", fr: "conversations qui ont reçu",
        de: "Unterhaltungen, die erhalten haben", it: "conversazioni che hanno ricevuto",
        ja: "受け取った会話", zh: "收到内容的对话", ko: "전달받은 대화",
        ru: "разговоры, получившие контекст" }
    /// Detail under the omitted figure.
    omitted_detail { en: "items that did not fit", pt: "itens que não couberam",
        es: "elementos que no cupieron", fr: "éléments qui n’ont pas tenu",
        de: "Einträge, die nicht passten", it: "elementi che non sono entrati",
        ja: "収まらなかった項目", zh: "放不下的条目", ko: "담기지 않은 항목",
        ru: "элементы, которые не поместились" }
    /// Label of the button that opens the delivery history.
    see_all_aria { en: "See all deliveries", pt: "Ver todas as entregas",
        es: "Ver todas las entregas", fr: "Voir toutes les livraisons",
        de: "Alle Lieferungen ansehen", it: "Vedi tutte le consegne",
        ja: "すべての配信を見る", zh: "查看全部交付", ko: "모든 전달 보기",
        ru: "Показать все доставки" }
    /// Button that opens the delivery history.
    see_all { en: "See all", pt: "Ver todas", es: "Ver todas", fr: "Tout voir",
        de: "Alle ansehen", it: "Vedi tutte", ja: "すべて見る", zh: "查看全部",
        ko: "모두 보기", ru: "Показать все" }

    // Deliveries page.

    /// Empty deliveries while the mode is off.
    deliveries_empty_off { en: "No deliveries: the mode is off. Choose Measure to see what would be sent, or Active to send it.",
        pt: "Nenhuma entrega: o modo está desligado. Escolha Medir para ver o que seria enviado, ou Ativo para enviar.",
        es: "Ninguna entrega: el modo está desactivado. Elige Medir para ver qué se enviaría, o Activo para enviarlo.",
        fr: "Aucune livraison : le mode est désactivé. Choisissez Mesurer pour voir ce qui serait envoyé, ou Actif pour l’envoyer.",
        de: "Keine Lieferungen: Der Modus ist aus. Wähle Messen, um zu sehen, was gesendet würde, oder Aktiv, um es zu senden.",
        it: "Nessuna consegna: la modalità è disattivata. Scegli Misura per vedere cosa verrebbe inviato, o Attivo per inviarlo.",
        ja: "配信はありません。モードがオフです。送信される内容を確認するには「計測」、送信するには「オン」を選んでください。",
        zh: "没有交付：模式已关闭。选择“测量”可查看将发送的内容，选择“启用”则会发送。",
        ko: "전달 내역이 없어요. 모드가 꺼져 있어요. 보냈을 내용을 보려면 '측정'을, 실제로 보내려면 '켬'을 선택해 주세요.",
        ru: "Доставок нет: режим выключен. Выберите «Измерять», чтобы увидеть, что было бы отправлено, или «Включено», чтобы отправлять." }
    /// Empty deliveries while a mode is on.
    deliveries_empty_on { en: "No deliveries yet. They appear when the agent makes a request in this project with the plugin connected.",
        pt: "Nenhuma entrega ainda. Elas aparecem quando o agente fizer um pedido neste projeto com o plugin conectado.",
        es: "Aún no hay entregas. Aparecen cuando el agente haga una solicitud en este proyecto con el plugin conectado.",
        fr: "Aucune livraison pour l’instant. Elles apparaissent quand l’agent fait une requête dans ce projet avec le plugin connecté.",
        de: "Noch keine Lieferungen. Sie erscheinen, sobald der Agent in diesem Projekt bei verbundenem Plugin eine Anfrage stellt.",
        it: "Ancora nessuna consegna. Compaiono quando l’agente fa una richiesta in questo progetto con il plugin connesso.",
        ja: "配信はまだありません。プラグインを接続した状態でエージェントがこのプロジェクトでリクエストすると表示されます。",
        zh: "还没有交付。当智能体在此项目中发出请求且插件已连接时，会显示在这里。",
        ko: "아직 전달 내역이 없어요. 플러그인이 연결된 상태에서 에이전트가 이 프로젝트에 요청하면 나타나요.",
        ru: "Доставок пока нет. Они появятся, когда агент сделает запрос в этом проекте при подключённом плагине." }
    /// Label of the button that opens the delivery mode page from the empty state.
    choose_mode_aria { en: "Choose the delivery mode", pt: "Escolher o modo de entrega",
        es: "Elegir el modo de entrega", fr: "Choisir le mode de livraison",
        de: "Lieferungsmodus wählen", it: "Scegli la modalità di consegna",
        ja: "配信モードを選ぶ", zh: "选择交付模式", ko: "전달 모드 선택",
        ru: "Выбрать режим доставки" }
    /// Button that opens the delivery mode page from the empty state.
    choose_mode { en: "Choose mode", pt: "Escolher modo", es: "Elegir modo",
        fr: "Choisir le mode", de: "Modus wählen", it: "Scegli modalità",
        ja: "モードを選ぶ", zh: "选择模式", ko: "모드 선택", ru: "Выбрать режим" }
    /// Status of a delivery that reached the agent.
    delivery_sent { en: "Sent", pt: "Enviado", es: "Enviado", fr: "Envoyé",
        de: "Gesendet", it: "Inviato", ja: "送信済み", zh: "已发送", ko: "전송됨",
        ru: "Отправлено" }
    /// Status of a delivery that was only measured.
    delivery_measured { en: "Measured", pt: "Medido", es: "Medido", fr: "Mesuré",
        de: "Gemessen", it: "Misurato", ja: "計測のみ", zh: "已测量", ko: "측정됨",
        ru: "Измерено" }
    /// Headline of a delivery without items.
    empty_block { en: "Empty block", pt: "Bloco vazio", es: "Bloque vacío",
        fr: "Bloc vide", de: "Leerer Block", it: "Blocco vuoto", ja: "空のブロック",
        zh: "空内容块", ko: "빈 블록", ru: "Пустой блок" }
    /// Item of a delivery whose source was deleted.
    item_missing { en: "Item that no longer exists", pt: "Item que não existe mais",
        es: "Elemento que ya no existe", fr: "Élément qui n’existe plus",
        de: "Eintrag, den es nicht mehr gibt", it: "Elemento che non esiste più",
        ja: "すでに存在しない項目", zh: "已不存在的条目", ko: "더 이상 없는 항목",
        ru: "Элемент, которого больше нет" }
    /// Kind of a delivered item that is a rule.
    item_rule { en: "rule", pt: "regra", es: "regla", fr: "règle", de: "Regel",
        it: "regola", ja: "ルール", zh: "规则", ko: "규칙", ru: "правило" }
    /// Shown when a delivery carried nothing.
    block_empty { en: "Nothing in the project matched the request; the block went out empty.",
        pt: "Nada do projeto casou com o pedido; o bloco saiu vazio.",
        es: "Nada del proyecto coincidió con la solicitud; el bloque salió vacío.",
        fr: "Rien dans le projet ne correspond à la requête ; le bloc est parti vide.",
        de: "Nichts aus dem Projekt passte zur Anfrage; der Block ging leer hinaus.",
        it: "Nulla del progetto corrisponde alla richiesta; il blocco è uscito vuoto.",
        ja: "リクエストに一致するプロジェクトの内容がなく、空のブロックになりました。",
        zh: "项目中没有与请求匹配的内容，内容块为空。",
        ko: "요청과 맞는 프로젝트 내용이 없어서 블록이 비어 있어요.",
        ru: "В проекте ничего не подошло под запрос; блок получился пустым." }

    // Shell of the screen.

    /// Eyebrow of the load error panel.
    context_word { en: "Context", pt: "Contexto", es: "Contexto", fr: "Contexte",
        de: "Kontext", it: "Contesto", ja: "コンテキスト", zh: "上下文",
        ko: "컨텍스트", ru: "Контекст" }
    /// Title of the load error panel.
    load_failed_title { en: "Could not load the context",
        pt: "Não foi possível carregar o contexto", es: "No se pudo cargar el contexto",
        fr: "Impossible de charger le contexte", de: "Der Kontext konnte nicht geladen werden",
        it: "Impossibile caricare il contesto", ja: "コンテキストを読み込めませんでした",
        zh: "无法加载上下文", ko: "컨텍스트를 불러오지 못했어요",
        ru: "Не удалось загрузить контекст" }
    /// Body of the load error panel.
    load_failed_body { en: "Try again; your data was not changed.",
        pt: "Tente de novo; seus dados não foram alterados.",
        es: "Inténtalo de nuevo; tus datos no se modificaron.",
        fr: "Réessayez ; vos données n’ont pas été modifiées.",
        de: "Versuche es erneut; deine Daten wurden nicht verändert.",
        it: "Riprova; i tuoi dati non sono stati modificati.",
        ja: "もう一度お試しください。データは変更されていません。",
        zh: "请重试，你的数据没有被更改。", ko: "다시 시도해 주세요. 데이터는 바뀌지 않았어요.",
        ru: "Повторите попытку; ваши данные не изменились." }
    /// Retry button of the error banner, and its label.
    retry { en: "Try again", pt: "Tentar de novo", es: "Reintentar", fr: "Réessayer",
        de: "Erneut versuchen", it: "Riprova", ja: "再試行", zh: "重试", ko: "다시 시도",
        ru: "Повторить" }
}

formats! {
    /// Notice after exporting the preview.
    preview_exported(path: &str) { en: "Preview exported to {path}",
        pt: "Prévia exportada para {path}", es: "Vista previa exportada a {path}",
        fr: "Aperçu exporté vers {path}", de: "Vorschau exportiert nach {path}",
        it: "Anteprima esportata in {path}", ja: "プレビューを {path} にエクスポートしました",
        zh: "预览已导出到 {path}", ko: "미리 보기를 {path}에 내보냈어요",
        ru: "Предпросмотр экспортирован в {path}" }
    /// Notice after reading the documents with nothing changed.
    documents_read_unchanged(total: usize) {
        en: "Documentation read: {total}, no changes.",
        pt: "Documentação lida: {total}, sem mudanças.",
        es: "Documentación leída: {total}, sin cambios.",
        fr: "Documentation lue : {total}, sans changement.",
        de: "Dokumentation gelesen: {total}, keine Änderungen.",
        it: "Documentazione letta: {total}, nessuna modifica.",
        ja: "ドキュメントを読み込みました: {total}件、変更なし。",
        zh: "已读取文档：{total} 个，没有变化。",
        ko: "문서를 읽었어요: {total}개, 변경 없음.",
        ru: "Документация прочитана: {total}, без изменений." }
    /// Notice after reading the documents with some changed.
    documents_read_changed(total: usize, changed: usize) {
        en: "Documentation read: {total}, {changed} changed.",
        pt: "Documentação lida: {total}, {changed} com mudanças.",
        es: "Documentación leída: {total}, {changed} con cambios.",
        fr: "Documentation lue : {total}, {changed} modifié(s).",
        de: "Dokumentation gelesen: {total}, davon {changed} geändert.",
        it: "Documentazione letta: {total}, {changed} con modifiche.",
        ja: "ドキュメントを読み込みました: {total}件、うち{changed}件に変更あり。",
        zh: "已读取文档：{total} 个，其中 {changed} 个有变化。",
        ko: "문서를 읽었어요: {total}개, {changed}개 변경됨.",
        ru: "Документация прочитана: {total}, изменений: {changed}." }
    /// Error when the typed budget is out of range.
    budget_out_of_range(min: usize, max: usize) {
        en: "Use a number between {min} and {max} tokens.",
        pt: "Use um número entre {min} e {max} tokens.",
        es: "Usa un número entre {min} y {max} tokens.",
        fr: "Utilisez un nombre entre {min} et {max} tokens.",
        de: "Verwende eine Zahl zwischen {min} und {max} Tokens.",
        it: "Usa un numero tra {min} e {max} token.",
        ja: "{min}〜{max}トークンの数値を入力してください。",
        zh: "请输入 {min} 到 {max} 之间的令牌数。",
        ko: "{min}에서 {max} 사이의 토큰 수를 입력해 주세요.",
        ru: "Введите число от {min} до {max} токенов." }
    /// Hint beside the budget field when it was saved.
    saved_on(date: &str) { en: "Saved on {date}.", pt: "Salvo em {date}.",
        es: "Guardado el {date}.", fr: "Enregistré le {date}.",
        de: "Gespeichert am {date}.", it: "Salvato il {date}.",
        ja: "{date}に保存しました。", zh: "保存于 {date}。", ko: "{date}에 저장했어요.",
        ru: "Сохранено {date}." }
    /// Hint beside the budget field when it is empty or being edited.
    budget_default(tokens: usize) {
        en: "Empty uses the default of {tokens} tokens.",
        pt: "Vazio usa o padrão de {tokens} tokens.",
        es: "Vacío usa el valor predeterminado de {tokens} tokens.",
        fr: "Vide, la valeur par défaut de {tokens} tokens s’applique.",
        de: "Leer gilt der Standard von {tokens} Tokens.",
        it: "Vuoto usa il valore predefinito di {tokens} token.",
        ja: "空欄の場合は既定の{tokens}トークンを使います。",
        zh: "留空则使用默认的 {tokens} 个令牌。",
        ko: "비워 두면 기본값인 {tokens}토큰을 써요.",
        ru: "Если оставить пустым, действует значение по умолчанию: {tokens} токенов." }
    /// Label of a decision row.
    open_decision_aria(question: &str) { en: "Open decision: {question}",
        pt: "Abrir decisão: {question}", es: "Abrir decisión: {question}",
        fr: "Ouvrir la décision : {question}", de: "Entscheidung öffnen: {question}",
        it: "Apri decisione: {question}", ja: "決定を開く: {question}",
        zh: "打开决策：{question}", ko: "결정 열기: {question}",
        ru: "Открыть решение: {question}" }
    /// Line after the documents of a group that were not listed.
    more_documents(more: usize) { en: "And {more} more", pt: "E mais {more}",
        es: "Y {more} más", fr: "Et {more} de plus", de: "Und {more} weitere",
        it: "E altri {more}", ja: "ほか{more}件", zh: "还有 {more} 个",
        ko: "외 {more}개", ru: "И ещё {more}" }
    /// Label of the button that shows more rules.
    show_more_rules_aria(hidden: usize) { en: "Show {hidden} more rules",
        pt: "Mostrar mais {hidden} regras", es: "Mostrar {hidden} reglas más",
        fr: "Afficher {hidden} règles de plus", de: "{hidden} weitere Regeln anzeigen",
        it: "Mostra altre {hidden} regole", ja: "さらに{hidden}件のルールを表示",
        zh: "再显示 {hidden} 条规则", ko: "규칙 {hidden}개 더 보기",
        ru: "Показать ещё правил: {hidden}" }
    /// Button that shows more rules.
    show_more(hidden: usize) { en: "Show {hidden} more", pt: "Mostrar mais {hidden}",
        es: "Mostrar {hidden} más", fr: "Afficher {hidden} de plus",
        de: "{hidden} weitere anzeigen", it: "Mostra altre {hidden}",
        ja: "さらに{hidden}件を表示", zh: "再显示 {hidden} 条", ko: "{hidden}개 더 보기",
        ru: "Показать ещё {hidden}" }
    /// Line of a rule with its inherited scope.
    inherited_scope(scope: &str) { en: "Inherited scope: {scope}",
        pt: "Escopo herdado: {scope}", es: "Alcance heredado: {scope}",
        fr: "Portée héritée : {scope}", de: "Geerbter Geltungsbereich: {scope}",
        it: "Ambito ereditato: {scope}", ja: "継承された範囲: {scope}",
        zh: "继承的范围：{scope}", ko: "상속된 범위: {scope}",
        ru: "Унаследованная область: {scope}" }
    /// Line of a decision of the preview with its scope.
    scope(scope: &str) { en: "Scope: {scope}", pt: "Escopo: {scope}",
        es: "Alcance: {scope}", fr: "Portée : {scope}",
        de: "Geltungsbereich: {scope}", it: "Ambito: {scope}", ja: "範囲: {scope}",
        zh: "范围：{scope}", ko: "범위: {scope}", ru: "Область: {scope}" }
    /// Hint of a rule: since when it applies.
    valid_since(date: &str) { en: "In effect since {date}", pt: "Vale desde {date}",
        es: "Vigente desde {date}", fr: "En vigueur depuis le {date}",
        de: "In Kraft seit {date}", it: "In vigore dal {date}", ja: "{date}から有効",
        zh: "自 {date} 起生效", ko: "{date}부터 유효", ru: "Действует с {date}" }
    /// Part of the preview summary: items left out of the budget.
    pack_omitted(omitted: usize) { en: "{omitted} out of budget",
        pt: "{omitted} fora do orçamento", es: "{omitted} fuera del presupuesto",
        fr: "{omitted} hors budget", de: "{omitted} außerhalb des Budgets",
        it: "{omitted} fuori budget", ja: "予算外 {omitted}件",
        zh: "{omitted} 项超出预算", ko: "예산 초과 {omitted}개",
        ru: "вне бюджета: {omitted}" }
    /// Size of the preview against its budget.
    chars_of(used: &str, budget: &str) { en: "{used} of {budget} characters",
        pt: "{used} de {budget} caracteres", es: "{used} de {budget} caracteres",
        fr: "{used} sur {budget} caractères", de: "{used} von {budget} Zeichen",
        it: "{used} di {budget} caratteri", ja: "{used} / {budget} 文字",
        zh: "{used} / {budget} 个字符", ko: "{used} / {budget}자",
        ru: "{used} из {budget} символов" }
    /// Kind of a preview rule that matched the task.
    kind_matched(kind: &str) { en: "{kind} · matched", pt: "{kind} · casou",
        es: "{kind} · coincidió", fr: "{kind} · correspondait", de: "{kind} · passte",
        it: "{kind} · corrispondeva", ja: "{kind} · 一致", zh: "{kind} · 已匹配",
        ko: "{kind} · 일치", ru: "{kind} · совпало" }
    /// Footer of the index: mode and budget.
    rail_foot(mode: &str, budget: usize) { en: "{mode} · up to {budget} tokens",
        pt: "{mode} · até {budget} tokens", es: "{mode} · hasta {budget} tokens",
        fr: "{mode} · jusqu’à {budget} tokens", de: "{mode} · bis zu {budget} Tokens",
        it: "{mode} · fino a {budget} token", ja: "{mode} · 最大{budget}トークン",
        zh: "{mode} · 最多 {budget} 个令牌", ko: "{mode} · 최대 {budget}토큰",
        ru: "{mode} · до {budget} токенов" }
    /// Pipeline stage III: the budget of a block.
    stage_budget(budget: usize) { en: "Up to {budget} tokens per block.",
        pt: "Até {budget} tokens por bloco.", es: "Hasta {budget} tokens por bloque.",
        fr: "Jusqu’à {budget} tokens par bloc.", de: "Bis zu {budget} Tokens pro Block.",
        it: "Fino a {budget} token per blocco.", ja: "1ブロックあたり最大{budget}トークン。",
        zh: "每个内容块最多 {budget} 个令牌。", ko: "블록당 최대 {budget}토큰.",
        ru: "До {budget} токенов на блок." }
    /// Detail under the deliveries figure.
    deliveries_detail(sent: usize, measured: usize) {
        en: "{sent} sent · {measured} measured",
        pt: "{sent} enviadas · {measured} medidas",
        es: "{sent} enviadas · {measured} medidas",
        fr: "{sent} envoyées · {measured} mesurées",
        de: "{sent} gesendet · {measured} gemessen",
        it: "{sent} inviate · {measured} misurate",
        ja: "送信 {sent}件 · 計測 {measured}件",
        zh: "已发送 {sent} · 已测量 {measured}",
        ko: "전송 {sent}건 · 측정 {measured}건",
        ru: "отправлено {sent} · измерено {measured}" }
    /// Detail under the tokens figure.
    tokens_detail(budget: usize) { en: "average · limit {budget}",
        pt: "média · limite {budget}", es: "media · límite {budget}",
        fr: "moyenne · limite {budget}", de: "Durchschnitt · Limit {budget}",
        it: "media · limite {budget}", ja: "平均 · 上限 {budget}",
        zh: "平均 · 上限 {budget}", ko: "평균 · 한도 {budget}",
        ru: "среднее · лимит {budget}" }
    /// Label of a delivery row for screen readers.
    delivery_aria(status: &str, time: &str, items: usize, tokens: usize, session: &str) {
        en: "{status} at {time}: {items} items, {tokens} tokens, session {session}",
        pt: "{status} às {time}: {items} itens, {tokens} tokens, sessão {session}",
        es: "{status} a las {time}: {items} elementos, {tokens} tokens, sesión {session}",
        fr: "{status} à {time} : {items} éléments, {tokens} tokens, session {session}",
        de: "{status} um {time}: {items} Einträge, {tokens} Tokens, Sitzung {session}",
        it: "{status} alle {time}: {items} elementi, {tokens} token, sessione {session}",
        ja: "{time}に{status}: 項目{items}件、{tokens}トークン、セッション {session}",
        zh: "{time} {status}：{items} 个条目，{tokens} 个令牌，会话 {session}",
        ko: "{time}에 {status}: 항목 {items}개, {tokens}토큰, 세션 {session}",
        ru: "{status} в {time}: элементов {items}, токенов {tokens}, сессия {session}" }
    /// Tokens of a delivery against the budget.
    tokens_of(tokens: usize, budget: usize) { en: "{tokens} / {budget} tokens",
        pt: "{tokens} / {budget} tokens", es: "{tokens} / {budget} tokens",
        fr: "{tokens} / {budget} tokens", de: "{tokens} / {budget} Tokens",
        it: "{tokens} / {budget} token", ja: "{tokens} / {budget} トークン",
        zh: "{tokens} / {budget} 个令牌", ko: "{tokens} / {budget}토큰",
        ru: "{tokens} / {budget} токенов" }
    /// Headline of a delivery: the first item and how many more.
    first_and_more(first: &str, more: usize) { en: "{first} and {more} more",
        pt: "{first} e mais {more}", es: "{first} y {more} más",
        fr: "{first} et {more} de plus", de: "{first} und {more} weitere",
        it: "{first} e altri {more}", ja: "{first} ほか{more}件",
        zh: "{first} 等另外 {more} 项", ko: "{first} 외 {more}개",
        ru: "{first} и ещё {more}" }
    /// Kind of a delivered item that is a decision.
    item_decision(version: i64) { en: "decision v{version}", pt: "decisão v{version}",
        es: "decisión v{version}", fr: "décision v{version}", de: "Entscheidung v{version}",
        it: "decisione v{version}", ja: "決定 v{version}", zh: "决策 v{version}",
        ko: "결정 v{version}", ru: "решение v{version}" }
    /// Last line of an expanded delivery.
    agent_session(session: &str) { en: "Agent session {session}",
        pt: "Sessão do agente {session}", es: "Sesión del agente {session}",
        fr: "Session de l’agent {session}", de: "Agent-Sitzung {session}",
        it: "Sessione dell’agente {session}", ja: "エージェントのセッション {session}",
        zh: "智能体会话 {session}", ko: "에이전트 세션 {session}",
        ru: "Сессия агента {session}" }
    /// Explanation of the shadow mode on the overview.
    mode_sentence_shadow(budget: usize) {
        en: "The block is computed and recorded in Deliveries, but does not go to the agent. Use it to see what would be sent (up to {budget} tokens) before turning it on.",
        pt: "O bloco é calculado e registrado em Entregas, mas não vai para o agente. Use para ver o que seria enviado (até {budget} tokens) antes de ativar.",
        es: "El bloque se calcula y se registra en Entregas, pero no va al agente. Úsalo para ver qué se enviaría (hasta {budget} tokens) antes de activarlo.",
        fr: "Le bloc est calculé et enregistré dans Livraisons, mais ne part pas vers l’agent. Utilisez-le pour voir ce qui serait envoyé (jusqu’à {budget} tokens) avant d’activer.",
        de: "Der Block wird berechnet und unter Lieferungen protokolliert, geht aber nicht an den Agent. Nutze das, um zu sehen, was gesendet würde (bis zu {budget} Tokens), bevor du aktivierst.",
        it: "Il blocco viene calcolato e registrato in Consegne, ma non va all’agente. Usalo per vedere cosa verrebbe inviato (fino a {budget} token) prima di attivare.",
        ja: "ブロックは計算されて配信に記録されますが、エージェントには送られません。オンにする前に、送信される内容(最大{budget}トークン)を確認するのに使えます。",
        zh: "内容块会被计算并记录在交付中，但不会发给智能体。可在启用前用它查看会发送什么（最多 {budget} 个令牌）。",
        ko: "블록을 계산해 전달 내역에 기록하지만 에이전트에는 보내지 않아요. 켜기 전에 보냈을 내용(최대 {budget}토큰)을 확인할 때 써요.",
        ru: "Блок рассчитывается и записывается в «Доставки», но агенту не отправляется. Используйте, чтобы увидеть, что было бы отправлено (до {budget} токенов), прежде чем включать." }
    /// Explanation of the active mode on the overview.
    mode_sentence_active(budget: usize) {
        en: "On every request and every edit, the agent receives a block of up to {budget} tokens with the decisions and rules that apply to that task.",
        pt: "A cada pedido e a cada edição, o agente recebe um bloco de até {budget} tokens com as decisões e regras que valem para aquela tarefa.",
        es: "En cada solicitud y cada edición, el agente recibe un bloque de hasta {budget} tokens con las decisiones y reglas que valen para esa tarea.",
        fr: "À chaque requête et à chaque modification, l’agent reçoit un bloc de {budget} tokens maximum avec les décisions et règles qui valent pour cette tâche.",
        de: "Bei jeder Anfrage und jeder Bearbeitung erhält der Agent einen Block von bis zu {budget} Tokens mit den Entscheidungen und Regeln, die für diese Aufgabe gelten.",
        it: "A ogni richiesta e a ogni modifica, l’agente riceve un blocco di al massimo {budget} token con le decisioni e le regole valide per quell’attività.",
        ja: "リクエストや編集のたびに、そのタスクに当てはまる決定とルールをまとめた最大{budget}トークンのブロックをエージェントが受け取ります。",
        zh: "每次请求和每次编辑，智能体都会收到一个最多 {budget} 个令牌的内容块，包含适用于该任务的决策和规则。",
        ko: "요청과 편집이 있을 때마다 에이전트는 해당 작업에 적용되는 결정과 규칙을 담은 최대 {budget}토큰의 블록을 받아요.",
        ru: "При каждом запросе и каждой правке агент получает блок до {budget} токенов с решениями и правилами, которые относятся к этой задаче." }
}

strings! {
    /// Explanation of the off mode on the overview.
    mode_sentence_off { en: "Nothing is computed or sent. Turn it on so the agent receives, with each request, the decisions and rules that matter for the task.",
        pt: "Nada é calculado nem enviado. Ative para que o agente receba, a cada pedido, as decisões e regras que importam para a tarefa.",
        es: "No se calcula ni se envía nada. Actívalo para que el agente reciba, en cada solicitud, las decisiones y reglas que importan para la tarea.",
        fr: "Rien n’est calculé ni envoyé. Activez-le pour que l’agent reçoive, à chaque requête, les décisions et règles qui comptent pour la tâche.",
        de: "Es wird nichts berechnet oder gesendet. Aktiviere es, damit der Agent bei jeder Anfrage die Entscheidungen und Regeln erhält, die für die Aufgabe wichtig sind.",
        it: "Non viene calcolato né inviato nulla. Attivalo perché l’agente riceva, a ogni richiesta, le decisioni e le regole che contano per l’attività.",
        ja: "計算も送信も行いません。オンにすると、リクエストのたびにタスクに関係する決定とルールをエージェントが受け取ります。",
        zh: "不计算，也不发送。启用后，智能体每次请求都会收到与任务相关的决策和规则。",
        ko: "계산도 전송도 하지 않아요. 켜면 요청할 때마다 에이전트가 작업에 중요한 결정과 규칙을 받아요.",
        ru: "Ничего не вычисляется и не отправляется. Включите, чтобы с каждым запросом агент получал решения и правила, важные для задачи." }
}
