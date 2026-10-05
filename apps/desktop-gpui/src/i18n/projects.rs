//! Copy for the projects area. See [`crate::i18n`] for how entries are declared.

use super::{formats, french_one, one, russian_form, strings, Language};

formats! {
    /// Accessible name of a project row.
    select_project(name: &str) {
        en: "Select {name}", pt: "Selecionar {name}", es: "Seleccionar {name}",
        fr: "Sélectionner {name}", de: "{name} auswählen", it: "Seleziona {name}",
        ja: "{name}を選択", zh: "选择 {name}", ko: "{name} 선택", ru: "Выбрать «{name}»" }
    /// Count chip while a search narrows the list.
    filtered_count(visible: usize, total: usize) {
        en: "{visible} of {total}", pt: "{visible} de {total}", es: "{visible} de {total}",
        fr: "{visible} sur {total}", de: "{visible} von {total}", it: "{visible} di {total}",
        ja: "{visible} / {total}", zh: "{visible} / {total}", ko: "{visible} / {total}",
        ru: "{visible} из {total}" }
    /// What typing the name erases, and what stays.
    purge_warning(name: &str) {
        en: "This can’t be undone. The folder on disk isn’t touched. Type {name} to confirm.",
        pt: "Não dá para desfazer. A pasta no disco não é tocada. Digite {name} para confirmar.",
        es: "No se puede deshacer. La carpeta en el disco no se toca. Escribe {name} para confirmar.",
        fr: "Impossible d’annuler. Le dossier sur le disque n’est pas touché. Saisissez {name} pour confirmer.",
        de: "Das lässt sich nicht rückgängig machen. Der Ordner auf der Festplatte bleibt unberührt. Gib {name} zur Bestätigung ein.",
        it: "Non si può annullare. La cartella sul disco non viene toccata. Scrivi {name} per confermare.",
        ja: "元に戻せません。ディスク上のフォルダには触れません。確定するには {name} と入力してください。",
        zh: "此操作无法撤销，磁盘上的文件夹不会被改动。请输入 {name} 以确认。",
        ko: "되돌릴 수 없어요. 디스크의 폴더는 건드리지 않아요. 확인하려면 {name}을(를) 입력하세요.",
        ru: "Это нельзя отменить. Папка на диске не затрагивается. Введите {name} для подтверждения." }
    /// Accessible name of the "search matched nothing" state.
    no_matches_aria(query: &str) {
        en: "No project matches {query}", pt: "Nenhum projeto corresponde a {query}",
        es: "Ningún proyecto coincide con {query}", fr: "Aucun projet ne correspond à {query}",
        de: "Kein Projekt entspricht {query}", it: "Nessun progetto corrisponde a {query}",
        ja: "{query}に一致するプロジェクトはありません", zh: "没有项目匹配 {query}",
        ko: "{query}와(과) 일치하는 프로젝트가 없어요", ru: "Нет проектов по запросу {query}" }
    /// Body of the "search matched nothing" state.
    no_matches_body(query: &str) {
        en: "No project matches “{query}”. Esc clears the search.",
        pt: "Nenhum projeto corresponde a “{query}”. Esc limpa a busca.",
        es: "Ningún proyecto coincide con “{query}”. Esc borra la búsqueda.",
        fr: "Aucun projet ne correspond à « {query} ». Échap efface la recherche.",
        de: "Kein Projekt entspricht „{query}“. Esc leert die Suche.",
        it: "Nessun progetto corrisponde a “{query}”. Esc cancella la ricerca.",
        ja: "「{query}」に一致するプロジェクトはありません。Esc で検索をクリアします。",
        zh: "没有项目匹配“{query}”。按 Esc 清除搜索。",
        ko: "“{query}”와(과) 일치하는 프로젝트가 없어요. Esc로 검색을 지워요.",
        ru: "Нет проектов по запросу «{query}». Esc очищает поиск." }
}

strings! {
    /// Title of the list-load failure.
    load_failed_title { en: "Couldn’t load the projects",
        pt: "Não foi possível carregar os projetos",
        es: "No se pudieron cargar los proyectos", fr: "Impossible de charger les projets",
        de: "Die Projekte konnten nicht geladen werden",
        it: "Impossibile caricare i progetti", ja: "プロジェクトを読み込めませんでした",
        zh: "无法加载项目", ko: "프로젝트를 불러오지 못했어요",
        ru: "Не удалось загрузить проекты" }
    /// Body of the list-load failure.
    load_failed_body { en: "Local storage didn’t respond; the list may be out of date.",
        pt: "O armazenamento local não respondeu; a lista pode estar desatualizada.",
        es: "El almacenamiento local no respondió; la lista puede estar desactualizada.",
        fr: "Le stockage local n’a pas répondu ; la liste est peut-être obsolète.",
        de: "Der lokale Speicher hat nicht geantwortet; die Liste ist möglicherweise veraltet.",
        it: "L’archivio locale non ha risposto; l’elenco potrebbe non essere aggiornato.",
        ja: "ローカルストレージが応答しませんでした。一覧が最新ではない可能性があります。",
        zh: "本地存储没有响应，列表可能已过期。",
        ko: "로컬 저장소가 응답하지 않았어요. 목록이 최신이 아닐 수 있어요.",
        ru: "Локальное хранилище не ответило; список мог устареть." }
    /// Title of the register/remove failure.
    action_failed_title { en: "Couldn’t complete the action",
        pt: "Não foi possível concluir a ação", es: "No se pudo completar la acción",
        fr: "Impossible de terminer l’action", de: "Die Aktion konnte nicht abgeschlossen werden",
        it: "Impossibile completare l’azione", ja: "操作を完了できませんでした",
        zh: "无法完成操作", ko: "작업을 완료하지 못했어요",
        ru: "Не удалось выполнить действие" }
    /// Body of the register/remove failure.
    action_failed_body { en: "Local storage refused the operation; nothing was changed.",
        pt: "O armazenamento local recusou a operação; nada foi alterado.",
        es: "El almacenamiento local rechazó la operación; no se cambió nada.",
        fr: "Le stockage local a refusé l’opération ; rien n’a été modifié.",
        de: "Der lokale Speicher hat den Vorgang abgelehnt; es wurde nichts geändert.",
        it: "L’archivio locale ha rifiutato l’operazione; non è cambiato nulla.",
        ja: "ローカルストレージが操作を拒否しました。何も変更されていません。",
        zh: "本地存储拒绝了该操作，没有任何改动。",
        ko: "로컬 저장소가 작업을 거부했어요. 바뀐 것은 없어요.",
        ru: "Локальное хранилище отклонило операцию; ничего не изменено." }
    /// Recovery hint of both storage failures.
    storage_recovery { en: "Try again. If the error continues, close and reopen the app.",
        pt: "Tente de novo. Se o erro continuar, feche e abra o app.",
        es: "Inténtalo de nuevo. Si el error continúa, cierra y abre la app.",
        fr: "Réessayez. Si l’erreur persiste, fermez et rouvrez l’app.",
        de: "Versuche es erneut. Wenn der Fehler bleibt, schließe die App und öffne sie neu.",
        it: "Riprova. Se l’errore continua, chiudi e riapri l’app.",
        ja: "もう一度お試しください。エラーが続く場合は、アプリを閉じて開き直してください。",
        zh: "请重试。如果错误仍然存在，请关闭并重新打开应用。",
        ko: "다시 시도해 보세요. 오류가 계속되면 앱을 닫았다가 다시 열어 주세요.",
        ru: "Повторите попытку. Если ошибка не уходит, закройте и снова откройте приложение." }
    /// Accessible context of the name-confirmation field.
    project_name_context { en: "Project name", pt: "Nome do projeto",
        es: "Nombre del proyecto", fr: "Nom du projet", de: "Projektname",
        it: "Nome del progetto", ja: "プロジェクト名", zh: "项目名称", ko: "프로젝트 이름",
        ru: "Название проекта" }
    /// Inline error: the project was already gone.
    project_already_gone { en: "That project was no longer on the list.",
        pt: "Esse projeto já não estava na lista.",
        es: "Ese proyecto ya no estaba en la lista.",
        fr: "Ce projet ne figurait déjà plus dans la liste.",
        de: "Dieses Projekt war nicht mehr in der Liste.",
        it: "Quel progetto non era più nell’elenco.",
        ja: "そのプロジェクトはすでに一覧にありませんでした。",
        zh: "该项目已不在列表中。", ko: "그 프로젝트는 이미 목록에 없었어요.",
        ru: "Этого проекта уже не было в списке." }
    /// Inline error: the typed name did not match.
    name_mismatch { en: "The name you typed doesn’t match; nothing was erased.",
        pt: "O nome digitado não confere; nada foi apagado.",
        es: "El nombre escrito no coincide; no se borró nada.",
        fr: "Le nom saisi ne correspond pas ; rien n’a été effacé.",
        de: "Der eingegebene Name stimmt nicht überein; nichts wurde gelöscht.",
        it: "Il nome digitato non corrisponde; non è stato cancellato nulla.",
        ja: "入力した名前が一致しません。何も削除されていません。",
        zh: "输入的名称不匹配，没有删除任何内容。",
        ko: "입력한 이름이 맞지 않아요. 지워진 것은 없어요.",
        ru: "Введённое название не совпадает; ничего не удалено." }
    /// Title of the system folder picker.
    picker_prompt { en: "Open folder", pt: "Abrir pasta", es: "Abrir carpeta",
        fr: "Ouvrir le dossier", de: "Ordner öffnen", it: "Apri cartella",
        ja: "フォルダーを開く", zh: "打开文件夹", ko: "폴더 열기", ru: "Открыть папку" }
    /// Inline error: the folder picker failed.
    picker_failed { en: "Couldn’t open the folder picker.",
        pt: "Não foi possível abrir o seletor de pastas.",
        es: "No se pudo abrir el selector de carpetas.",
        fr: "Impossible d’ouvrir le sélecteur de dossiers.",
        de: "Die Ordnerauswahl konnte nicht geöffnet werden.",
        it: "Impossibile aprire il selettore di cartelle.",
        ja: "フォルダー選択を開けませんでした。", zh: "无法打开文件夹选择器。",
        ko: "폴더 선택기를 열지 못했어요.", ru: "Не удалось открыть выбор папки." }
    /// Accessible name of the open-folder buttons.
    open_folder_aria { en: "Open folder in the system picker",
        pt: "Abrir pasta no seletor do sistema",
        es: "Abrir carpeta en el selector del sistema",
        fr: "Ouvrir un dossier dans le sélecteur du système",
        de: "Ordner in der Systemauswahl öffnen",
        it: "Apri cartella nel selettore di sistema",
        ja: "システムの選択画面でフォルダーを開く", zh: "在系统选择器中打开文件夹",
        ko: "시스템 선택기에서 폴더 열기", ru: "Открыть папку в системном окне выбора" }
    /// Open-folder button and tooltip.
    open_folder { en: "Open folder…", pt: "Abrir pasta…", es: "Abrir carpeta…",
        fr: "Ouvrir un dossier…", de: "Ordner öffnen …", it: "Apri cartella…",
        ja: "フォルダーを開く…", zh: "打开文件夹…", ko: "폴더 열기…", ru: "Открыть папку…" }
    /// Accessible name of the first-run state.
    empty_aria { en: "No tracked projects", pt: "Nenhum projeto acompanhado",
        es: "Ningún proyecto en seguimiento", fr: "Aucun projet suivi",
        de: "Keine verfolgten Projekte", it: "Nessun progetto monitorato",
        ja: "追跡中のプロジェクトはありません", zh: "没有正在跟踪的项目",
        ko: "추적 중인 프로젝트 없음", ru: "Нет отслеживаемых проектов" }
    /// Section label of the first-run state.
    first_project { en: "First project", pt: "Primeiro projeto", es: "Primer proyecto",
        fr: "Premier projet", de: "Erstes Projekt", it: "Primo progetto",
        ja: "最初のプロジェクト", zh: "第一个项目", ko: "첫 프로젝트", ru: "Первый проект" }
    /// Title of the first-run state.
    empty_title { en: "Start with a folder", pt: "Comece por uma pasta",
        es: "Empieza por una carpeta", fr: "Commencez par un dossier",
        de: "Beginne mit einem Ordner", it: "Inizia da una cartella",
        ja: "フォルダーから始めましょう", zh: "从一个文件夹开始", ko: "폴더로 시작해 보세요",
        ru: "Начните с папки" }
    /// Body of the first-run state.
    empty_body { en: "Choose an existing folder to track. Your files stay where they are.",
        pt: "Escolha uma pasta existente para acompanhar. Seus arquivos permanecem no lugar.",
        es: "Elige una carpeta existente para seguirla. Tus archivos se quedan donde están.",
        fr: "Choisissez un dossier existant à suivre. Vos fichiers restent là où ils sont.",
        de: "Wähle einen vorhandenen Ordner zum Verfolgen. Deine Dateien bleiben, wo sie sind.",
        it: "Scegli una cartella esistente da monitorare. I tuoi file restano dove sono.",
        ja: "追跡する既存のフォルダーを選んでください。ファイルはそのままの場所に残ります。",
        zh: "选择一个现有文件夹来跟踪。你的文件会保留在原处。",
        ko: "추적할 기존 폴더를 고르세요. 파일은 그대로 그 자리에 있어요.",
        ru: "Выберите существующую папку для отслеживания. Ваши файлы останутся на месте." }
    /// Title of the projects panel.
    projects_title { en: "Projects", pt: "Projetos", es: "Proyectos", fr: "Projets",
        de: "Projekte", it: "Progetti", ja: "プロジェクト", zh: "项目", ko: "프로젝트",
        ru: "Проекты" }
    /// Question before removing a project from the list.
    remove_question { en: "Remove from the list? The folder stays on disk.",
        pt: "Remover da lista? A pasta permanece no disco.",
        es: "¿Quitar de la lista? La carpeta permanece en el disco.",
        fr: "Retirer de la liste ? Le dossier reste sur le disque.",
        de: "Aus der Liste entfernen? Der Ordner bleibt auf der Festplatte.",
        it: "Rimuovere dall’elenco? La cartella resta sul disco.",
        ja: "一覧から削除しますか？フォルダーはディスクに残ります。",
        zh: "要从列表中移除吗？文件夹会保留在磁盘上。",
        ko: "목록에서 제거할까요? 폴더는 디스크에 그대로 남아요.",
        ru: "Убрать из списка? Папка останется на диске." }
    /// Cancel button.
    cancel { en: "Cancel", pt: "Cancelar", es: "Cancelar", fr: "Annuler",
        de: "Abbrechen", it: "Annulla", ja: "キャンセル", zh: "取消", ko: "취소",
        ru: "Отмена" }
    /// Confirm-removal button.
    remove { en: "Remove", pt: "Remover", es: "Quitar", fr: "Retirer", de: "Entfernen",
        it: "Rimuovi", ja: "削除", zh: "移除", ko: "제거", ru: "Убрать" }
    /// Remove-from-list button.
    remove_from_list { en: "Remove from list", pt: "Remover da lista",
        es: "Quitar de la lista", fr: "Retirer de la liste",
        de: "Aus der Liste entfernen", it: "Rimuovi dall’elenco", ja: "一覧から削除",
        zh: "从列表中移除", ko: "목록에서 제거", ru: "Убрать из списка" }
    /// Erase-data button.
    purge_data { en: "Erase data…", pt: "Apagar dados…", es: "Borrar datos…",
        fr: "Effacer les données…", de: "Daten löschen …", it: "Cancella i dati…",
        ja: "データを削除…", zh: "删除数据…", ko: "데이터 지우기…", ru: "Стереть данные…" }
    /// Section label of the project panel.
    project_label { en: "Project", pt: "Projeto", es: "Proyecto", fr: "Projet",
        de: "Projekt", it: "Progetto", ja: "プロジェクト", zh: "项目", ko: "프로젝트",
        ru: "Проект" }
    /// Detail row: where the project lives.
    location { en: "Location", pt: "Localização", es: "Ubicación", fr: "Emplacement",
        de: "Speicherort", it: "Posizione", ja: "場所", zh: "位置", ko: "위치",
        ru: "Расположение" }
    /// Detail row: when the project was added.
    added { en: "Added", pt: "Adicionado", es: "Añadido", fr: "Ajouté", de: "Hinzugefügt",
        it: "Aggiunto", ja: "追加日", zh: "添加时间", ko: "추가됨", ru: "Добавлен" }
    /// Shown while the erase impact is measured.
    measuring { en: "Measuring what will be erased…", pt: "Medindo o que será apagado…",
        es: "Midiendo lo que se borrará…", fr: "Mesure de ce qui sera effacé…",
        de: "Es wird gemessen, was gelöscht wird …",
        it: "Sto misurando cosa verrà cancellato…",
        ja: "削除される内容を確認しています…", zh: "正在统计将被删除的内容…",
        ko: "지워질 내용을 확인하는 중…", ru: "Подсчитываю, что будет стёрто…" }
    /// Shown when the project holds no data.
    no_data { en: "This project has no data yet; only its registration will be erased.",
        pt: "Este projeto ainda não tem dados; só o cadastro será apagado.",
        es: "Este proyecto aún no tiene datos; solo se borrará el registro.",
        fr: "Ce projet n’a pas encore de données ; seul son enregistrement sera effacé.",
        de: "Dieses Projekt hat noch keine Daten; nur der Eintrag wird gelöscht.",
        it: "Questo progetto non ha ancora dati; verrà cancellata solo la registrazione.",
        ja: "このプロジェクトにはまだデータがありません。登録だけが削除されます。",
        zh: "该项目还没有数据，只会删除其登记信息。",
        ko: "이 프로젝트에는 아직 데이터가 없어요. 등록 정보만 지워져요.",
        ru: "В этом проекте пока нет данных; будет стёрта только запись о нём." }
    /// Title of the erase confirmation.
    purge_title { en: "Erase the project and all its data?",
        pt: "Apagar o projeto e todos os dados?",
        es: "¿Borrar el proyecto y todos sus datos?",
        fr: "Effacer le projet et toutes ses données ?",
        de: "Das Projekt und alle Daten löschen?",
        it: "Cancellare il progetto e tutti i dati?",
        ja: "プロジェクトとすべてのデータを削除しますか？",
        zh: "要删除该项目及其所有数据吗？", ko: "프로젝트와 모든 데이터를 지울까요?",
        ru: "Стереть проект и все его данные?" }
    /// Confirm-erase button and its accessible name.
    purge_all { en: "Erase everything", pt: "Apagar tudo", es: "Borrar todo",
        fr: "Tout effacer", de: "Alles löschen", it: "Cancella tutto",
        ja: "すべて削除", zh: "全部删除", ko: "모두 지우기", ru: "Стереть всё" }
    /// Title of the "search matched nothing" state.
    no_matches_title { en: "Nothing matches", pt: "Nada corresponde",
        es: "Nada coincide", fr: "Aucun résultat", de: "Nichts passt",
        it: "Nessuna corrispondenza", ja: "一致するものがありません",
        zh: "没有匹配项", ko: "일치하는 항목이 없어요", ru: "Ничего не найдено" }
    /// Accessible description of the loading dot.
    loading_projects { en: "Loading projects", pt: "Carregando projetos",
        es: "Cargando proyectos", fr: "Chargement des projets",
        de: "Projekte werden geladen", it: "Caricamento dei progetti",
        ja: "プロジェクトを読み込み中", zh: "正在加载项目", ko: "프로젝트를 불러오는 중",
        ru: "Загружаю проекты" }
    /// Inline error: the path is not a directory.
    invalid_location { en: "That path doesn’t exist or isn’t a directory.",
        pt: "Esse caminho não existe ou não é um diretório.",
        es: "Esa ruta no existe o no es un directorio.",
        fr: "Ce chemin n’existe pas ou n’est pas un répertoire.",
        de: "Dieser Pfad existiert nicht oder ist kein Verzeichnis.",
        it: "Quel percorso non esiste o non è una directory.",
        ja: "そのパスは存在しないか、ディレクトリではありません。",
        zh: "该路径不存在，或不是目录。", ko: "그 경로가 없거나 디렉터리가 아니에요.",
        ru: "Этот путь не существует или не является каталогом." }
    /// Inline error: the directory is already tracked.
    already_registered { en: "That directory is already being tracked.",
        pt: "Esse diretório já está sendo acompanhado.",
        es: "Ese directorio ya está en seguimiento.",
        fr: "Ce répertoire est déjà suivi.",
        de: "Dieses Verzeichnis wird bereits verfolgt.",
        it: "Quella directory è già monitorata.",
        ja: "そのディレクトリはすでに追跡中です。", zh: "该目录已在跟踪中。",
        ko: "그 디렉터리는 이미 추적 중이에요.", ru: "Этот каталог уже отслеживается." }
    /// Inline error: the project does not exist.
    project_not_found { en: "That project wasn’t found.",
        pt: "Esse projeto não foi encontrado.", es: "No se encontró ese proyecto.",
        fr: "Ce projet est introuvable.", de: "Dieses Projekt wurde nicht gefunden.",
        it: "Quel progetto non è stato trovato.", ja: "そのプロジェクトは見つかりませんでした。",
        zh: "未找到该项目。", ko: "그 프로젝트를 찾지 못했어요.",
        ru: "Этот проект не найден." }
}

fn pick(n: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if one(n) {
        singular
    } else {
        plural
    }
}

fn pick_french(n: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if french_one(n) {
        singular
    } else {
        plural
    }
}

fn pick_russian(n: usize, forms: [&'static str; 3]) -> &'static str {
    forms[russian_form(n)]
}

/// Row label in the erase preview: decisions with versions and relations.
pub fn impact_decisions(n: usize) -> &'static str {
    match super::current() {
        Language::English => pick(
            n,
            "decision with versions and relations",
            "decisions with versions and relations",
        ),
        Language::Portuguese => pick(
            n,
            "decisão com versões e relações",
            "decisões com versões e relações",
        ),
        Language::Spanish => pick(
            n,
            "decisión con versiones y relaciones",
            "decisiones con versiones y relaciones",
        ),
        Language::French => pick_french(
            n,
            "décision avec versions et relations",
            "décisions avec versions et relations",
        ),
        Language::German => pick(
            n,
            "Entscheidung mit Versionen und Beziehungen",
            "Entscheidungen mit Versionen und Beziehungen",
        ),
        Language::Italian => pick(
            n,
            "decisione con versioni e relazioni",
            "decisioni con versioni e relazioni",
        ),
        Language::Japanese => "バージョンと関係を持つ決定",
        Language::Chinese => "含版本和关系的决策",
        Language::Korean => "버전과 관계가 있는 결정",
        Language::Russian => pick_russian(
            n,
            [
                "решение с версиями и связями",
                "решения с версиями и связями",
                "решений с версиями и связями",
            ],
        ),
    }
}

/// Row label in the erase preview: candidates.
pub fn impact_candidates(n: usize) -> &'static str {
    match super::current() {
        Language::English => pick(n, "candidate", "candidates"),
        Language::Portuguese => pick(n, "candidato", "candidatos"),
        Language::Spanish => pick(n, "candidato", "candidatos"),
        Language::French => pick_french(n, "candidat", "candidats"),
        Language::German => pick(n, "Kandidat", "Kandidaten"),
        Language::Italian => pick(n, "candidato", "candidati"),
        Language::Japanese => "候補",
        Language::Chinese => "候选",
        Language::Korean => "후보",
        Language::Russian => pick_russian(n, ["кандидат", "кандидата", "кандидатов"]),
    }
}

/// Row label in the erase preview: project rules.
pub fn impact_claims(n: usize) -> &'static str {
    match super::current() {
        Language::English => pick(n, "project rule", "project rules"),
        Language::Portuguese => pick(n, "regra do projeto", "regras do projeto"),
        Language::Spanish => pick(n, "regla del proyecto", "reglas del proyecto"),
        Language::French => pick_french(n, "règle du projet", "règles du projet"),
        Language::German => pick(n, "Projektregel", "Projektregeln"),
        Language::Italian => pick(n, "regola del progetto", "regole del progetto"),
        Language::Japanese => "プロジェクトのルール",
        Language::Chinese => "项目规则",
        Language::Korean => "프로젝트 규칙",
        Language::Russian => {
            pick_russian(n, ["правило проекта", "правила проекта", "правил проекта"])
        }
    }
}

/// Row label in the erase preview: captures with evidence.
pub fn impact_captures(n: usize) -> &'static str {
    match super::current() {
        Language::English => pick(n, "capture with evidence", "captures with evidence"),
        Language::Portuguese => pick(n, "captura com evidências", "capturas com evidências"),
        Language::Spanish => pick(n, "captura con evidencias", "capturas con evidencias"),
        Language::French => pick_french(n, "capture avec preuves", "captures avec preuves"),
        Language::German => pick(n, "Erfassung mit Belegen", "Erfassungen mit Belegen"),
        Language::Italian => pick(n, "acquisizione con evidenze", "acquisizioni con evidenze"),
        Language::Japanese => "根拠付きのキャプチャ",
        Language::Chinese => "带证据的捕获",
        Language::Korean => "근거가 있는 캡처",
        Language::Russian => pick_russian(
            n,
            [
                "захват с доказательствами",
                "захвата с доказательствами",
                "захватов с доказательствами",
            ],
        ),
    }
}

/// Row label in the erase preview: context delivery records.
pub fn impact_injections(n: usize) -> &'static str {
    match super::current() {
        Language::English => pick(n, "sent context record", "sent context records"),
        Language::Portuguese => pick(
            n,
            "registro de contexto enviado",
            "registros de contexto enviado",
        ),
        Language::Spanish => pick(
            n,
            "registro de contexto enviado",
            "registros de contexto enviado",
        ),
        Language::French => pick_french(
            n,
            "enregistrement de contexte envoyé",
            "enregistrements de contexte envoyé",
        ),
        Language::German => pick(n, "gesendeter Kontexteintrag", "gesendete Kontexteinträge"),
        Language::Italian => pick(
            n,
            "registro di contesto inviato",
            "registri di contesto inviato",
        ),
        Language::Japanese => "送信したコンテキストの記録",
        Language::Chinese => "已发送的上下文记录",
        Language::Korean => "전송한 컨텍스트 기록",
        Language::Russian => pick_russian(
            n,
            [
                "запись отправленного контекста",
                "записи отправленного контекста",
                "записей отправленного контекста",
            ],
        ),
    }
}
