//! Copy for the settings area. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings, Language};

strings! {
    /// Settings section and card: the interface language.
    language_title { en: "Language", pt: "Idioma", es: "Idioma", fr: "Langue",
        de: "Sprache", it: "Lingua", ja: "言語", zh: "语言", ko: "언어", ru: "Язык" }
    /// Subtitle of the language section.
    language_subtitle { en: "The language of every screen.",
        pt: "O idioma de todas as telas.",
        es: "El idioma de todas las pantallas.",
        fr: "La langue de tous les écrans.",
        de: "Die Sprache aller Ansichten.",
        it: "La lingua di tutte le schermate.",
        ja: "すべての画面の表示言語。",
        zh: "所有界面的显示语言。",
        ko: "모든 화면의 표시 언어.",
        ru: "Язык всех экранов." }
    /// What the language card does.
    language_card_body {
        en: "Changes right away and is saved for next time. What you write and capture stays as written.",
        pt: "Muda na hora e fica salvo para a próxima vez. O que você escreve e captura continua como foi escrito.",
        es: "Cambia al instante y se guarda para la próxima vez. Lo que escribes y capturas queda tal como se escribió.",
        fr: "S’applique tout de suite et reste enregistré. Ce que vous écrivez et capturez reste tel quel.",
        de: "Gilt sofort und bleibt gespeichert. Was du schreibst und erfasst, bleibt so, wie es geschrieben wurde.",
        it: "Cambia subito e resta salvata per la prossima volta. Ciò che scrivi e catturi resta com’è stato scritto.",
        ja: "すぐに切り替わり、次回も保持されます。あなたが書いた内容や取り込んだ内容はそのまま残ります。",
        zh: "立即生效，并保存到下次使用。你写下和捕获的内容保持原样。",
        ko: "즉시 바뀌고 다음에도 유지됩니다. 직접 쓰거나 수집한 내용은 쓴 그대로 남습니다.",
        ru: "Меняется сразу и сохраняется на следующий раз. То, что вы пишете и сохраняете, остаётся как есть." }
    /// Palette entry that opens the language section.
    language_palette { en: "Change language", pt: "Mudar idioma", es: "Cambiar idioma",
        fr: "Changer de langue", de: "Sprache ändern", it: "Cambia lingua",
        ja: "言語を変更", zh: "更改语言", ko: "언어 변경", ru: "Сменить язык" }
}

formats! {
    /// Where a palette entry leads inside Settings: `Settings › Language`.
    settings_at(section: &str) { en: "Settings › {section}", pt: "Configurações › {section}",
        es: "Ajustes › {section}", fr: "Réglages › {section}", de: "Einstellungen › {section}",
        it: "Impostazioni › {section}", ja: "設定 › {section}", zh: "设置 › {section}",
        ko: "설정 › {section}", ru: "Настройки › {section}" }
}

/// `language` named in the interface language: `Portuguese (Brazil)` while
/// the app reads English. The picker shows it beside the native name.
pub fn language_name(language: Language) -> &'static str {
    use Language::*;
    let names: [&'static str; 10] = match super::current() {
        English => [
            "English",
            "Portuguese (Brazil)",
            "Spanish",
            "French",
            "German",
            "Italian",
            "Japanese",
            "Chinese (Simplified)",
            "Korean",
            "Russian",
        ],
        Portuguese => [
            "Inglês",
            "Português (Brasil)",
            "Espanhol",
            "Francês",
            "Alemão",
            "Italiano",
            "Japonês",
            "Chinês (simplificado)",
            "Coreano",
            "Russo",
        ],
        Spanish => [
            "Inglés",
            "Portugués (Brasil)",
            "Español",
            "Francés",
            "Alemán",
            "Italiano",
            "Japonés",
            "Chino (simplificado)",
            "Coreano",
            "Ruso",
        ],
        French => [
            "Anglais",
            "Portugais (Brésil)",
            "Espagnol",
            "Français",
            "Allemand",
            "Italien",
            "Japonais",
            "Chinois (simplifié)",
            "Coréen",
            "Russe",
        ],
        German => [
            "Englisch",
            "Portugiesisch (Brasilien)",
            "Spanisch",
            "Französisch",
            "Deutsch",
            "Italienisch",
            "Japanisch",
            "Chinesisch (vereinfacht)",
            "Koreanisch",
            "Russisch",
        ],
        Italian => [
            "Inglese",
            "Portoghese (Brasile)",
            "Spagnolo",
            "Francese",
            "Tedesco",
            "Italiano",
            "Giapponese",
            "Cinese (semplificato)",
            "Coreano",
            "Russo",
        ],
        Japanese => [
            "英語",
            "ポルトガル語（ブラジル）",
            "スペイン語",
            "フランス語",
            "ドイツ語",
            "イタリア語",
            "日本語",
            "中国語（簡体字）",
            "韓国語",
            "ロシア語",
        ],
        Chinese => [
            "英语",
            "葡萄牙语（巴西）",
            "西班牙语",
            "法语",
            "德语",
            "意大利语",
            "日语",
            "简体中文",
            "韩语",
            "俄语",
        ],
        Korean => [
            "영어",
            "포르투갈어(브라질)",
            "스페인어",
            "프랑스어",
            "독일어",
            "이탈리아어",
            "일본어",
            "중국어(간체)",
            "한국어",
            "러시아어",
        ],
        Russian => [
            "Английский",
            "Португальский (Бразилия)",
            "Испанский",
            "Французский",
            "Немецкий",
            "Итальянский",
            "Японский",
            "Китайский (упрощённый)",
            "Корейский",
            "Русский",
        ],
    };
    names[language as usize]
}

/// Declares a counted noun: one `fn name(n: usize) -> String` per entry. English,
/// Portuguese, Spanish, German and Italian give (singular, plural); French (singular
/// for 0 and 1, plural); Russian (one, few, many); Japanese, Chinese and Korean one
/// template. Every template names `{n}`.
macro_rules! counted {
    ($(
        $(#[$meta:meta])*
        $name:ident {
            en: ($en1:literal, $en2:literal), pt: ($pt1:literal, $pt2:literal),
            es: ($es1:literal, $es2:literal), fr: ($fr1:literal, $fr2:literal),
            de: ($de1:literal, $de2:literal), it: ($it1:literal, $it2:literal),
            ja: $ja:literal, zh: $zh:literal, ko: $ko:literal,
            ru: ($ru1:literal, $ru2:literal, $ru5:literal) $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        pub fn $name(n: usize) -> String {
            use super::{french_one, one, russian_form};
            match super::current() {
                Language::English => {
                    if one(n) { format!($en1, n = n) } else { format!($en2, n = n) }
                }
                Language::Portuguese => {
                    if one(n) { format!($pt1, n = n) } else { format!($pt2, n = n) }
                }
                Language::Spanish => {
                    if one(n) { format!($es1, n = n) } else { format!($es2, n = n) }
                }
                Language::French => {
                    if french_one(n) { format!($fr1, n = n) } else { format!($fr2, n = n) }
                }
                Language::German => {
                    if one(n) { format!($de1, n = n) } else { format!($de2, n = n) }
                }
                Language::Italian => {
                    if one(n) { format!($it1, n = n) } else { format!($it2, n = n) }
                }
                Language::Japanese => format!($ja, n = n),
                Language::Chinese => format!($zh, n = n),
                Language::Korean => format!($ko, n = n),
                Language::Russian => match russian_form(n) {
                    0 => format!($ru1, n = n),
                    1 => format!($ru2, n = n),
                    _ => format!($ru5, n = n),
                },
            }
        }
    )*};
}

strings! {
    /// Button that retries a failed load.
    action_try_again {
        en: "Try again",
        pt: "Tentar de novo",
        es: "Reintentar",
        fr: "Réessayer",
        de: "Erneut versuchen",
        it: "Riprova",
        ja: "再試行",
        zh: "重试",
        ko: "다시 시도",
        ru: "Повторить",
    }
    /// Button that cancels a pending action.
    action_cancel {
        en: "Cancel",
        pt: "Cancelar",
        es: "Cancelar",
        fr: "Annuler",
        de: "Abbrechen",
        it: "Annulla",
        ja: "キャンセル",
        zh: "取消",
        ko: "취소",
        ru: "Отмена",
    }
    /// Button that sends a failed item through analysis again.
    action_reprocess {
        en: "Reprocess",
        pt: "Reprocessar",
        es: "Reprocesar",
        fr: "Retraiter",
        de: "Neu verarbeiten",
        it: "Rielabora",
        ja: "再処理",
        zh: "重新处理",
        ko: "재처리",
        ru: "Обработать заново",
    }
    /// Screen reader label of the button that leaves Settings.
    nav_back_to_projects {
        en: "Back to projects",
        pt: "Voltar aos projetos",
        es: "Volver a proyectos",
        fr: "Retour aux projets",
        de: "Zurück zu den Projekten",
        it: "Torna ai progetti",
        ja: "プロジェクトに戻る",
        zh: "返回项目",
        ko: "프로젝트로 돌아가기",
        ru: "Назад к проектам",
    }
    /// Visible label of the button that leaves Settings.
    nav_projects {
        en: "Projects",
        pt: "Projetos",
        es: "Proyectos",
        fr: "Projets",
        de: "Projekte",
        it: "Progetti",
        ja: "プロジェクト",
        zh: "项目",
        ko: "프로젝트",
        ru: "Проекты",
    }
    /// Heading of the Settings navigation, in capitals.
    nav_settings_label {
        en: "SETTINGS",
        pt: "CONFIGURAÇÕES",
        es: "AJUSTES",
        fr: "RÉGLAGES",
        de: "EINSTELLUNGEN",
        it: "IMPOSTAZIONI",
        ja: "設定",
        zh: "设置",
        ko: "설정",
        ru: "НАСТРОЙКИ",
    }
    /// Settings section: extraction provider and privacy.
    section_ai_title {
        en: "AI and privacy",
        pt: "IA e privacidade",
        es: "IA y privacidad",
        fr: "IA et confidentialité",
        de: "KI und Datenschutz",
        it: "IA e privacy",
        ja: "AIとプライバシー",
        zh: "AI 与隐私",
        ko: "AI와 개인정보",
        ru: "ИИ и приватность",
    }
    /// Subtitle of the AI and privacy section.
    section_ai_subtitle {
        en: "How captures become candidates and what can leave this machine.",
        pt: "Como as capturas viram candidatos e o que pode sair desta máquina.",
        es: "Cómo las capturas se convierten en candidatos y qué puede salir de esta máquina.",
        fr: "Comment les captures deviennent des candidats et ce qui peut quitter cette machine.",
        de: "Wie Erfassungen zu Kandidaten werden und was diesen Rechner verlassen darf.",
        it: "Come le acquisizioni diventano candidati e che cosa può uscire da questa macchina.",
        ja: "キャプチャが候補になる流れと、このマシンから外に出る可能性のあるもの。",
        zh: "捕获如何变成候选，以及哪些内容可能离开这台设备。",
        ko: "캡처가 후보가 되는 과정과 이 컴퓨터 밖으로 나갈 수 있는 것을 보여줘요.",
        ru: "Как захваты становятся кандидатами и что может покинуть этот компьютер.",
    }
    /// Subtitle of the OpenCode section.
    section_opencode_subtitle {
        en: "Whether OpenCode captures are arriving and, if not, why.",
        pt: "Se as capturas do OpenCode estão chegando e, se não, por quê.",
        es: "Si las capturas de OpenCode están llegando y, si no, por qué.",
        fr: "Si les captures d’OpenCode arrivent et, sinon, pourquoi.",
        de: "Ob OpenCode-Erfassungen ankommen und, falls nicht, warum.",
        it: "Se le acquisizioni di OpenCode stanno arrivando e, in caso contrario, perché.",
        ja: "OpenCodeのキャプチャが届いているか、届いていない場合はその理由。",
        zh: "OpenCode 的捕获是否正常到达；如果没有，原因是什么。",
        ko: "OpenCode 캡처가 도착하고 있는지, 아니라면 이유가 무엇인지 보여줘요.",
        ru: "Приходят ли захваты из OpenCode и, если нет, почему.",
    }
    /// Settings section: pipeline health.
    section_diagnostics_title {
        en: "Diagnostics",
        pt: "Diagnóstico",
        es: "Diagnóstico",
        fr: "Diagnostic",
        de: "Diagnose",
        it: "Diagnostica",
        ja: "診断",
        zh: "诊断",
        ko: "진단",
        ru: "Диагностика",
    }
    /// Subtitle of the diagnostics section.
    section_diagnostics_subtitle {
        en: "How the pipeline is doing and what was lost along the way.",
        pt: "Como o pipeline está indo e o que se perdeu no caminho.",
        es: "Cómo va el pipeline y qué se perdió en el camino.",
        fr: "Où en est le pipeline et ce qui s’est perdu en route.",
        de: "Wie die Pipeline läuft und was unterwegs verloren ging.",
        it: "Come sta andando la pipeline e che cosa si è perso lungo il percorso.",
        ja: "パイプラインの状況と、途中で失われたもの。",
        zh: "管道运行得如何，以及途中丢失了什么。",
        ko: "파이프라인이 어떻게 돌아가는지와 중간에 잃은 것을 보여줘요.",
        ru: "Как работает конвейер и что потерялось по пути.",
    }
    /// Settings section: theme and background.
    section_appearance_title {
        en: "Appearance",
        pt: "Aparência",
        es: "Apariencia",
        fr: "Apparence",
        de: "Erscheinungsbild",
        it: "Aspetto",
        ja: "外観",
        zh: "外观",
        ko: "모양",
        ru: "Внешний вид",
    }
    /// Subtitle of the appearance section.
    section_appearance_subtitle {
        en: "The app theme and the background behind the surfaces.",
        pt: "O tema do app e o fundo atrás das superfícies.",
        es: "El tema de la app y el fondo detrás de las superficies.",
        fr: "Le thème de l’app et le fond derrière les surfaces.",
        de: "Das Design der App und der Hintergrund hinter den Flächen.",
        it: "Il tema dell’app e lo sfondo dietro le superfici.",
        ja: "アプリのテーマと、面の後ろに表示する背景。",
        zh: "应用主题，以及界面后方的背景。",
        ko: "앱 테마와 화면 뒤에 깔리는 배경이에요.",
        ru: "Тема приложения и фон за поверхностями.",
    }
    /// Confirmation text before revoking consent and deleting the API key.
    ai_revoke_warning {
        en: "This turns off external calls and deletes the key from the system credential \
            store. The address and model stay saved.",
        pt: "Isso desliga as chamadas externas e apaga a chave do cofre. O endereço e o modelo \
            continuam salvos.",
        es: "Esto desactiva las llamadas externas y borra la clave del almacén de credenciales \
            del sistema. La dirección y el modelo siguen guardados.",
        fr: "Cela désactive les appels externes et supprime la clé du coffre d’identifiants du \
            système. L’adresse et le modèle restent enregistrés.",
        de: "Das schaltet externe Aufrufe ab und löscht den Schlüssel aus dem \
            Anmeldeinformationsspeicher des Systems. Adresse und Modell bleiben gespeichert.",
        it: "Disattiva le chiamate esterne ed elimina la chiave dall’archivio credenziali di \
            sistema. L’indirizzo e il modello restano salvati.",
        ja: "外部への呼び出しをオフにし、システムの資格情報ストアからキーを削除します。アドレスとモデルは保存されたままです。",
        zh: "这将关闭外部调用，并从系统凭据存储中删除密钥。地址和模型会继续保留。",
        ko: "외부 호출을 끄고 시스템 자격 증명 저장소에서 키를 삭제해요. 주소와 모델은 그대로 저장돼 있어요.",
        ru: "Внешние вызовы будут отключены, а ключ удалён из системного хранилища учётных \
            данных. Адрес и модель останутся сохранёнными.",
    }
    /// Confirmation text before revoking consent when no key is deleted.
    ai_revoke_only_warning {
        en: "This turns off external calls. The configuration and the account stay saved.",
        pt: "Isso desliga as chamadas externas. A configuração e a conta continuam salvas.",
        es: "Esto desactiva las llamadas externas. La configuración y la cuenta siguen guardadas.",
        fr: "Cela désactive les appels externes. La configuration et le compte restent \
            enregistrés.",
        de: "Das schaltet externe Aufrufe ab. Die Konfiguration und das Konto bleiben gespeichert.",
        it: "Disattiva le chiamate esterne. La configurazione e l’account restano salvati.",
        ja: "外部への呼び出しをオフにします。設定とアカウントは保存されたままです。",
        zh: "这将关闭外部调用。配置和账户会继续保留。",
        ko: "외부 호출을 꺼요. 설정과 계정은 그대로 저장돼 있어요.",
        ru: "Внешние вызовы будут отключены. Настройка и учётная запись останутся сохранёнными.",
    }
    /// Preview card text when the local extractor is chosen.
    ai_local_only {
        en: "With the local extractor, no capture content leaves this machine.",
        pt: "Com o extrator local, nenhum conteúdo das capturas sai desta máquina.",
        es: "Con el extractor local, ningún contenido de las capturas sale de esta máquina.",
        fr: "Avec l’extracteur local, aucun contenu des captures ne quitte cette machine.",
        de: "Mit dem lokalen Extraktor verlässt kein Erfassungsinhalt diesen Rechner.",
        it: "Con l’estrattore locale, nessun contenuto delle acquisizioni esce da questa macchina.",
        ja: "ローカル抽出を使うと、キャプチャの内容がこのマシンの外に出ることはありません。",
        zh: "使用本地提取器时，捕获内容不会离开这台设备。",
        ko: "로컬 추출기를 쓰면 캡처 내용이 이 컴퓨터 밖으로 나가지 않아요.",
        ru: "При локальном извлечении содержимое захватов не покидает этот компьютер.",
    }
    /// Card text: where the provider key lives.
    ai_key_description {
        en: "Stored in the system credential manager, never in a file, and not shown here.",
        pt: "Fica no Gerenciador de Credenciais do sistema, nunca em arquivo, e não é exibida \
            aqui.",
        es: "Se guarda en el administrador de credenciales del sistema, nunca en un archivo, y \
            no se muestra aquí.",
        fr: "Elle est conservée dans le gestionnaire d’identifiants du système, jamais dans un \
            fichier, et n’est pas affichée ici.",
        de: "Sie liegt in der Anmeldeinformationsverwaltung des Systems, nie in einer Datei, \
            und wird hier nicht angezeigt.",
        it: "Resta nella gestione credenziali di sistema, mai in un file, e non viene mostrata \
            qui.",
        ja: "システムの資格情報マネージャーに保存され、ファイルには残らず、ここには表示されません。",
        zh: "保存在系统凭据管理器中，绝不写入文件，也不会在此显示。",
        ko: "시스템 자격 증명 관리자에 저장되며, 파일에는 남기지 않고 여기에도 표시하지 않아요.",
        ru: "Хранится в системном менеджере учётных данных, никогда в файле, и здесь не \
            показывается.",
    }
    /// Footer hint: saving invalidates the current consent.
    ai_consent_invalidation {
        en: "Saving changes the preview: the current consent stops applying until you consent \
            again.",
        pt: "Salvar muda a prévia: o consentimento atual deixa de valer até você consentir de \
            novo.",
        es: "Guardar cambia la vista previa: el consentimiento actual deja de valer hasta que \
            vuelvas a consentir.",
        fr: "Enregistrer modifie l’aperçu : le consentement actuel cesse de s’appliquer jusqu’à \
            ce que vous consentiez de nouveau.",
        de: "Speichern ändert die Vorschau: Die aktuelle Einwilligung gilt nicht mehr, bis du \
            erneut einwilligst.",
        it: "Salvare cambia l’anteprima: il consenso attuale non vale più finché non acconsenti \
            di nuovo.",
        ja: "保存するとプレビューが変わり、再度同意するまで現在の同意は無効になります。",
        zh: "保存会改变预览：在你重新同意之前，当前的同意将不再有效。",
        ko: "저장하면 미리 보기가 바뀌어서, 다시 동의하기 전까지 지금의 동의는 효력이 없어져요.",
        ru: "Сохранение меняет предпросмотр: текущее согласие перестанет действовать, пока вы \
            не дадите его снова.",
    }
    /// Preview card text: secrets are redacted at capture time.
    ai_redaction_on {
        en: "Secrets are redacted at capture, before anything is sent.",
        pt: "Segredos são redigidos na captura, antes de qualquer envio.",
        es: "Los secretos se ocultan en la captura, antes de cualquier envío.",
        fr: "Les secrets sont masqués à la capture, avant tout envoi.",
        de: "Geheimnisse werden bei der Erfassung geschwärzt, bevor etwas gesendet wird.",
        it: "I segreti vengono oscurati all’acquisizione, prima di qualsiasi invio.",
        ja: "シークレットはキャプチャ時に伏せ字にされ、送信前に取り除かれます。",
        zh: "机密信息在捕获时即被遮盖，早于任何发送。",
        ko: "비밀 정보는 캡처할 때 가려지며, 어떤 전송보다도 먼저 처리돼요.",
        ru: "Секреты скрываются при захвате, до любой отправки.",
    }
    /// Preview card text: secrets are not redacted at capture time.
    ai_redaction_off {
        en: "No secret redaction at capture.",
        pt: "Sem redação de segredos na captura.",
        es: "Sin ocultar secretos en la captura.",
        fr: "Pas de masquage des secrets à la capture.",
        de: "Keine Schwärzung von Geheimnissen bei der Erfassung.",
        it: "Nessun oscuramento dei segreti all’acquisizione.",
        ja: "キャプチャ時にシークレットを伏せ字にしません。",
        zh: "捕获时不遮盖机密信息。",
        ko: "캡처할 때 비밀 정보를 가리지 않아요.",
        ru: "Секреты при захвате не скрываются.",
    }
    /// Label of the provider address field.
    field_endpoint_label {
        en: "Provider address",
        pt: "Endereço do provedor",
        es: "Dirección del proveedor",
        fr: "Adresse du fournisseur",
        de: "Adresse des Anbieters",
        it: "Indirizzo del provider",
        ja: "プロバイダーのアドレス",
        zh: "提供商地址",
        ko: "공급자 주소",
        ru: "Адрес провайдера",
    }
    /// Label of the model field.
    field_model_label {
        en: "Model",
        pt: "Modelo",
        es: "Modelo",
        fr: "Modèle",
        de: "Modell",
        it: "Modello",
        ja: "モデル",
        zh: "模型",
        ko: "모델",
        ru: "Модель",
    }
    /// Label of the model field for the ChatGPT plan.
    field_plan_model_label {
        en: "Plan model",
        pt: "Modelo do plano",
        es: "Modelo del plan",
        fr: "Modèle du forfait",
        de: "Modell des Tarifs",
        it: "Modello del piano",
        ja: "プランのモデル",
        zh: "套餐模型",
        ko: "플랜 모델",
        ru: "Модель тарифа",
    }
    /// Label of the per-item character limit field.
    field_limit_label {
        en: "Character limit per item",
        pt: "Limite de caracteres por item",
        es: "Límite de caracteres por elemento",
        fr: "Limite de caractères par élément",
        de: "Zeichenlimit pro Element",
        it: "Limite di caratteri per elemento",
        ja: "項目ごとの文字数の上限",
        zh: "每项的字符上限",
        ko: "항목당 글자 수 한도",
        ru: "Лимит символов на элемент",
    }
    /// Placeholder of the provider address field.
    field_endpoint_placeholder {
        en: "https://api.example.com/v1",
        pt: "https://api.exemplo.com/v1",
        es: "https://api.ejemplo.com/v1",
        fr: "https://api.exemple.com/v1",
        de: "https://api.beispiel.com/v1",
        it: "https://api.esempio.com/v1",
        ja: "https://api.example.com/v1",
        zh: "https://api.example.com/v1",
        ko: "https://api.example.com/v1",
        ru: "https://api.example.com/v1",
    }
    /// Placeholder of the model field.
    field_model_placeholder {
        en: "Model name at the provider",
        pt: "Nome do modelo no provedor",
        es: "Nombre del modelo en el proveedor",
        fr: "Nom du modèle chez le fournisseur",
        de: "Modellname beim Anbieter",
        it: "Nome del modello presso il provider",
        ja: "プロバイダー上のモデル名",
        zh: "提供商处的模型名称",
        ko: "공급자의 모델 이름",
        ru: "Название модели у провайдера",
    }
    /// Placeholder of the provider API key field.
    field_key_placeholder {
        en: "Provider API key",
        pt: "Chave de API do provedor",
        es: "Clave de API del proveedor",
        fr: "Clé d’API du fournisseur",
        de: "API-Schlüssel des Anbieters",
        it: "Chiave API del provider",
        ja: "プロバイダーのAPIキー",
        zh: "提供商的 API 密钥",
        ko: "공급자 API 키",
        ru: "API-ключ провайдера",
    }
    /// Placeholder of the OpenCode API key field.
    field_opencode_key_placeholder {
        en: "OpenCode API key",
        pt: "Chave de API do OpenCode",
        es: "Clave de API de OpenCode",
        fr: "Clé d’API OpenCode",
        de: "OpenCode-API-Schlüssel",
        it: "Chiave API di OpenCode",
        ja: "OpenCodeのAPIキー",
        zh: "OpenCode 的 API 密钥",
        ko: "OpenCode API 키",
        ru: "API-ключ OpenCode",
    }
    /// Error: the form was used before the profile loaded.
    ai_err_not_loaded {
        en: "The configuration hasn’t loaded yet.",
        pt: "Configuração ainda não carregada.",
        es: "La configuración aún no se ha cargado.",
        fr: "La configuration n’est pas encore chargée.",
        de: "Die Konfiguration ist noch nicht geladen.",
        it: "La configurazione non è ancora stata caricata.",
        ja: "設定がまだ読み込まれていません。",
        zh: "配置尚未加载。",
        ko: "설정을 아직 불러오지 않았어요.",
        ru: "Настройка ещё не загружена.",
    }
    /// Error: the per-item limit is not a whole positive number.
    ai_err_limit {
        en: "The per-item limit must be a positive whole number.",
        pt: "O limite por item deve ser um número inteiro positivo.",
        es: "El límite por elemento debe ser un número entero positivo.",
        fr: "La limite par élément doit être un nombre entier positif.",
        de: "Das Limit pro Element muss eine positive ganze Zahl sein.",
        it: "Il limite per elemento deve essere un numero intero positivo.",
        ja: "項目ごとの上限は正の整数で指定してください。",
        zh: "每项的上限必须是正整数。",
        ko: "항목당 한도는 양의 정수여야 해요.",
        ru: "Лимит на элемент должен быть целым положительным числом.",
    }
    /// Error: the settings file or the system credential store could not be accessed.
    ai_err_io {
        en: "Couldn’t access the settings or the system credential store. Try again.",
        pt: "Não foi possível acessar as configurações ou o cofre do sistema. Tente de novo.",
        es: "No se pudo acceder a los ajustes ni al almacén de credenciales del sistema. \
            Inténtalo de nuevo.",
        fr: "Impossible d’accéder aux réglages ou au coffre d’identifiants du système. Réessayez.",
        de: "Auf die Einstellungen oder den Anmeldeinformationsspeicher des Systems konnte \
            nicht zugegriffen werden. Versuche es erneut.",
        it: "Impossibile accedere alle impostazioni o all’archivio credenziali di sistema. \
            Riprova.",
        ja: "設定またはシステムの資格情報ストアにアクセスできませんでした。もう一度お試しください。",
        zh: "无法访问设置或系统凭据存储。请重试。",
        ko: "설정이나 시스템 자격 증명 저장소에 접근할 수 없어요. 다시 시도해 주세요.",
        ru: "Не удалось получить доступ к настройкам или системному хранилищу учётных данных. \
            Повторите попытку.",
    }
    /// Toast: the local extractor was activated.
    ai_notice_local_on {
        en: "Local extraction on. No content leaves this machine.",
        pt: "Extração local ativada. Nenhum conteúdo sai desta máquina.",
        es: "Extracción local activada. Ningún contenido sale de esta máquina.",
        fr: "Extraction locale activée. Aucun contenu ne quitte cette machine.",
        de: "Lokale Extraktion aktiviert. Kein Inhalt verlässt diesen Rechner.",
        it: "Estrazione locale attivata. Nessun contenuto esce da questa macchina.",
        ja: "ローカル抽出をオンにしました。内容がこのマシンの外に出ることはありません。",
        zh: "已启用本地提取。没有内容会离开这台设备。",
        ko: "로컬 추출을 켰어요. 내용이 이 컴퓨터 밖으로 나가지 않아요.",
        ru: "Локальное извлечение включено. Содержимое не покидает этот компьютер.",
    }
    /// Toast: the provider configuration was saved.
    ai_notice_saved {
        en: "Configuration saved. Review the preview and consent to turn it on.",
        pt: "Configuração salva. Revise a prévia e consinta para ativar.",
        es: "Configuración guardada. Revisa la vista previa y da tu consentimiento para activarla.",
        fr: "Configuration enregistrée. Relisez l’aperçu et donnez votre consentement pour \
            l’activer.",
        de: "Konfiguration gespeichert. Prüfe die Vorschau und willige ein, um sie zu aktivieren.",
        it: "Configurazione salvata. Controlla l’anteprima e dai il consenso per attivarla.",
        ja: "設定を保存しました。プレビューを確認し、同意して有効にしてください。",
        zh: "配置已保存。请查看预览并同意后启用。",
        ko: "설정을 저장했어요. 미리 보기를 확인하고 동의하면 켜져요.",
        ru: "Настройка сохранена. Проверьте предпросмотр и дайте согласие, чтобы включить.",
    }
    /// Toast: the API key went into the system credential store.
    ai_notice_key_stored {
        en: "Key stored in the system credential store.",
        pt: "Chave guardada no cofre do sistema.",
        es: "Clave guardada en el almacén de credenciales del sistema.",
        fr: "Clé enregistrée dans le coffre d’identifiants du système.",
        de: "Schlüssel im Anmeldeinformationsspeicher des Systems gespeichert.",
        it: "Chiave salvata nell’archivio credenziali di sistema.",
        ja: "キーをシステムの資格情報ストアに保存しました。",
        zh: "密钥已保存到系统凭据存储。",
        ko: "키를 시스템 자격 증명 저장소에 저장했어요.",
        ru: "Ключ сохранён в системном хранилище учётных данных.",
    }
    /// Toast: consent was recorded.
    ai_notice_consent_granted {
        en: "Consent recorded. The external provider is on.",
        pt: "Consentimento registrado. O provedor externo está ativo.",
        es: "Consentimiento registrado. El proveedor externo está activo.",
        fr: "Consentement enregistré. Le fournisseur externe est actif.",
        de: "Einwilligung gespeichert. Der externe Anbieter ist aktiv.",
        it: "Consenso registrato. Il provider esterno è attivo.",
        ja: "同意を記録しました。外部プロバイダーが有効です。",
        zh: "已记录同意。外部提供商已启用。",
        ko: "동의를 기록했어요. 외부 공급자가 켜져 있어요.",
        ru: "Согласие записано. Внешний провайдер включён.",
    }
    /// Toast: external calls off and the key deleted.
    ai_notice_revoked_with_key {
        en: "External calls turned off and key deleted from the system credential store.",
        pt: "Chamadas externas desligadas e chave apagada do cofre.",
        es: "Llamadas externas desactivadas y clave borrada del almacén de credenciales.",
        fr: "Appels externes désactivés et clé supprimée du coffre d’identifiants.",
        de: "Externe Aufrufe abgeschaltet und Schlüssel aus dem Anmeldeinformationsspeicher \
            gelöscht.",
        it: "Chiamate esterne disattivate e chiave eliminata dall’archivio credenziali.",
        ja: "外部への呼び出しをオフにし、キーを資格情報ストアから削除しました。",
        zh: "已关闭外部调用，并从凭据存储中删除密钥。",
        ko: "외부 호출을 끄고 자격 증명 저장소에서 키를 삭제했어요.",
        ru: "Внешние вызовы отключены, ключ удалён из хранилища учётных данных.",
    }
    /// Toast: external calls off.
    ai_notice_revoked {
        en: "External calls turned off.",
        pt: "Chamadas externas desligadas.",
        es: "Llamadas externas desactivadas.",
        fr: "Appels externes désactivés.",
        de: "Externe Aufrufe abgeschaltet.",
        it: "Chiamate esterne disattivate.",
        ja: "外部への呼び出しをオフにしました。",
        zh: "已关闭外部调用。",
        ko: "외부 호출을 껐어요.",
        ru: "Внешние вызовы отключены.",
    }
    /// Status line: local extraction.
    ai_status_local_title {
        en: "Local extraction, no network",
        pt: "Extração local, sem rede",
        es: "Extracción local, sin red",
        fr: "Extraction locale, sans réseau",
        de: "Lokale Extraktion, ohne Netzwerk",
        it: "Estrazione locale, senza rete",
        ja: "ローカル抽出（ネットワークなし）",
        zh: "本地提取，无需联网",
        ko: "로컬 추출, 네트워크 없음",
        ru: "Локальное извлечение, без сети",
    }
    /// Status line body: local extraction.
    ai_status_local_body {
        en: "Candidates are extracted on this machine. No capture content is sent.",
        pt: "Candidatos são extraídos nesta máquina. Nenhum conteúdo das capturas é enviado.",
        es: "Los candidatos se extraen en esta máquina. No se envía ningún contenido de las \
            capturas.",
        fr: "Les candidats sont extraits sur cette machine. Aucun contenu des captures n’est \
            envoyé.",
        de: "Kandidaten werden auf diesem Rechner extrahiert. Es wird kein Erfassungsinhalt \
            gesendet.",
        it: "I candidati vengono estratti su questa macchina. Nessun contenuto delle \
            acquisizioni viene inviato.",
        ja: "候補はこのマシン上で抽出されます。キャプチャの内容は送信されません。",
        zh: "候选在这台设备上提取。不会发送任何捕获内容。",
        ko: "후보를 이 컴퓨터에서 추출해요. 캡처 내용은 전송하지 않아요.",
        ru: "Кандидаты извлекаются на этом компьютере. Содержимое захватов не отправляется.",
    }
    /// Status line: external provider on.
    ai_status_active_title {
        en: "External provider on",
        pt: "Provedor externo ativo",
        es: "Proveedor externo activo",
        fr: "Fournisseur externe actif",
        de: "Externer Anbieter aktiv",
        it: "Provider esterno attivo",
        ja: "外部プロバイダーが有効",
        zh: "外部提供商已启用",
        ko: "외부 공급자 켜짐",
        ru: "Внешний провайдер включён",
    }
    /// Status line: external provider blocked.
    ai_status_blocked_title {
        en: "External provider blocked",
        pt: "Provedor externo bloqueado",
        es: "Proveedor externo bloqueado",
        fr: "Fournisseur externe bloqué",
        de: "Externer Anbieter blockiert",
        it: "Provider esterno bloccato",
        ja: "外部プロバイダーはブロック中",
        zh: "外部提供商已被阻止",
        ko: "외부 공급자 차단됨",
        ru: "Внешний провайдер заблокирован",
    }
    /// Why the provider is blocked: consent missing.
    ai_blocked_consent {
        en: "You still need to consent to the preview below.",
        pt: "Falta consentir com a prévia abaixo.",
        es: "Falta dar tu consentimiento a la vista previa de abajo.",
        fr: "Il reste à consentir à l’aperçu ci-dessous.",
        de: "Du musst noch der Vorschau unten zustimmen.",
        it: "Manca il consenso all’anteprima qui sotto.",
        ja: "下のプレビューへの同意がまだです。",
        zh: "还需要同意下方的预览。",
        ko: "아래 미리 보기에 아직 동의하지 않았어요.",
        ru: "Осталось дать согласие на предпросмотр ниже.",
    }
    /// Why the provider is blocked: configuration changed after consent.
    ai_blocked_changed {
        en: "The configuration changed after consent; consent again.",
        pt: "A configuração mudou depois do consentimento; consinta de novo.",
        es: "La configuración cambió tras el consentimiento; vuelve a consentir.",
        fr: "La configuration a changé après le consentement ; consentez de nouveau.",
        de: "Die Konfiguration hat sich nach der Einwilligung geändert; willige erneut ein.",
        it: "La configurazione è cambiata dopo il consenso; acconsenti di nuovo.",
        ja: "同意のあとで設定が変わりました。もう一度同意してください。",
        zh: "同意之后配置发生了变化；请重新同意。",
        ko: "동의한 뒤에 설정이 바뀌었어요. 다시 동의해 주세요.",
        ru: "Настройка изменилась после согласия; дайте согласие снова.",
    }
    /// Why the provider is blocked: invalid provider configuration.
    ai_blocked_invalid {
        en: "The provider configuration is incomplete or invalid.",
        pt: "A configuração do provedor está incompleta ou inválida.",
        es: "La configuración del proveedor está incompleta o no es válida.",
        fr: "La configuration du fournisseur est incomplète ou invalide.",
        de: "Die Konfiguration des Anbieters ist unvollständig oder ungültig.",
        it: "La configurazione del provider è incompleta o non valida.",
        ja: "プロバイダーの設定が不完全か、正しくありません。",
        zh: "提供商配置不完整或无效。",
        ko: "공급자 설정이 덜 채워졌거나 올바르지 않아요.",
        ru: "Настройка провайдера неполная или недействительная.",
    }
    /// Extractor option: local heuristic.
    ai_kind_local_title {
        en: "Local",
        pt: "Local",
        es: "Local",
        fr: "Local",
        de: "Lokal",
        it: "Locale",
        ja: "ローカル",
        zh: "本地",
        ko: "로컬",
        ru: "Локально",
    }
    /// Description of the local extractor option.
    ai_kind_local_body {
        en: "Offline heuristic. Nothing leaves this machine.",
        pt: "Heurística offline. Nada sai desta máquina.",
        es: "Heurística sin conexión. Nada sale de esta máquina.",
        fr: "Heuristique hors ligne. Rien ne quitte cette machine.",
        de: "Offline-Heuristik. Nichts verlässt diesen Rechner.",
        it: "Euristica offline. Nulla esce da questa macchina.",
        ja: "オフラインのヒューリスティック。何もこのマシンの外に出ません。",
        zh: "离线启发式。没有任何内容离开这台设备。",
        ko: "오프라인 휴리스틱이에요. 아무것도 이 컴퓨터 밖으로 나가지 않아요.",
        ru: "Офлайн-эвристика. Ничто не покидает этот компьютер.",
    }
    /// Extractor option: local model or API.
    ai_kind_remote_title {
        en: "Local model or API",
        pt: "Modelo local ou API",
        es: "Modelo local o API",
        fr: "Modèle local ou API",
        de: "Lokales Modell oder API",
        it: "Modello locale o API",
        ja: "ローカルモデルまたはAPI",
        zh: "本地模型或 API",
        ko: "로컬 모델 또는 API",
        ru: "Локальная модель или API",
    }
    /// Description of the local model or API extractor option.
    ai_kind_remote_body {
        en: "Ollama, LM Studio or an OpenAI-compatible address.",
        pt: "Ollama, LM Studio ou um endereço compatível com OpenAI.",
        es: "Ollama, LM Studio o una dirección compatible con OpenAI.",
        fr: "Ollama, LM Studio ou une adresse compatible OpenAI.",
        de: "Ollama, LM Studio oder eine OpenAI-kompatible Adresse.",
        it: "Ollama, LM Studio o un indirizzo compatibile con OpenAI.",
        ja: "Ollama、LM Studio、またはOpenAI互換のアドレス。",
        zh: "Ollama、LM Studio 或与 OpenAI 兼容的地址。",
        ko: "Ollama, LM Studio 또는 OpenAI 호환 주소예요.",
        ru: "Ollama, LM Studio или адрес, совместимый с OpenAI.",
    }
    /// Extractor option: ChatGPT account.
    ai_kind_chatgpt_title {
        en: "ChatGPT account",
        pt: "Conta ChatGPT",
        es: "Cuenta de ChatGPT",
        fr: "Compte ChatGPT",
        de: "ChatGPT-Konto",
        it: "Account ChatGPT",
        ja: "ChatGPTアカウント",
        zh: "ChatGPT 账户",
        ko: "ChatGPT 계정",
        ru: "Учётная запись ChatGPT",
    }
    /// Description of the ChatGPT account extractor option.
    ai_kind_chatgpt_body {
        en: "Uses your ChatGPT plan, no API key.",
        pt: "Usa o seu plano do ChatGPT, sem chave de API.",
        es: "Usa tu plan de ChatGPT, sin clave de API.",
        fr: "Utilise votre forfait ChatGPT, sans clé d’API.",
        de: "Nutzt deinen ChatGPT-Tarif, ohne API-Schlüssel.",
        it: "Usa il tuo piano ChatGPT, senza chiave API.",
        ja: "ChatGPTのプランを使います。APIキーは不要です。",
        zh: "使用你的 ChatGPT 套餐，无需 API 密钥。",
        ko: "ChatGPT 플랜을 사용해요. API 키는 필요 없어요.",
        ru: "Использует ваш тариф ChatGPT, API-ключ не нужен.",
    }
    /// Extractor option: OpenCode Zen or Go.
    ai_kind_opencode_title {
        en: "OpenCode Zen or Go",
        pt: "OpenCode Zen ou Go",
        es: "OpenCode Zen o Go",
        fr: "OpenCode Zen ou Go",
        de: "OpenCode Zen oder Go",
        it: "OpenCode Zen o Go",
        ja: "OpenCode ZenまたはGo",
        zh: "OpenCode Zen 或 Go",
        ko: "OpenCode Zen 또는 Go",
        ru: "OpenCode Zen или Go",
    }
    /// Description of the OpenCode extractor option.
    ai_kind_opencode_body {
        en: "Paste your OpenCode key and pick a model.",
        pt: "Cole a chave do OpenCode e escolha um modelo.",
        es: "Pega tu clave de OpenCode y elige un modelo.",
        fr: "Collez votre clé OpenCode et choisissez un modèle.",
        de: "Füge deinen OpenCode-Schlüssel ein und wähle ein Modell.",
        it: "Incolla la chiave di OpenCode e scegli un modello.",
        ja: "OpenCodeのキーを貼り付けて、モデルを選んでください。",
        zh: "粘贴你的 OpenCode 密钥并选择模型。",
        ko: "OpenCode 키를 붙여넣고 모델을 고르세요.",
        ru: "Вставьте ключ OpenCode и выберите модель.",
    }
    /// Save button label while saving.
    ai_saving {
        en: "Saving…",
        pt: "Salvando…",
        es: "Guardando…",
        fr: "Enregistrement…",
        de: "Wird gespeichert…",
        it: "Salvataggio…",
        ja: "保存中…",
        zh: "正在保存…",
        ko: "저장 중…",
        ru: "Сохранение…",
    }
    /// Save button label.
    ai_save {
        en: "Save configuration",
        pt: "Salvar configuração",
        es: "Guardar configuración",
        fr: "Enregistrer la configuration",
        de: "Konfiguration speichern",
        it: "Salva configurazione",
        ja: "設定を保存",
        zh: "保存配置",
        ko: "설정 저장",
        ru: "Сохранить настройку",
    }
    /// Hint: the ChatGPT plan needs a model before saving.
    ai_hint_pick_plan_model {
        en: "Pick a plan model to save.",
        pt: "Escolha um modelo do plano para salvar.",
        es: "Elige un modelo del plan para guardar.",
        fr: "Choisissez un modèle du forfait pour enregistrer.",
        de: "Wähle ein Modell des Tarifs, um zu speichern.",
        it: "Scegli un modello del piano per salvare.",
        ja: "保存するには、プランのモデルを選んでください。",
        zh: "请选择套餐模型后再保存。",
        ko: "저장하려면 플랜 모델을 고르세요.",
        ru: "Выберите модель тарифа, чтобы сохранить.",
    }
    /// Hint: OpenCode needs a model before saving.
    ai_hint_pick_opencode_model {
        en: "Pick an OpenCode model to save.",
        pt: "Escolha um modelo do OpenCode para salvar.",
        es: "Elige un modelo de OpenCode para guardar.",
        fr: "Choisissez un modèle OpenCode pour enregistrer.",
        de: "Wähle ein OpenCode-Modell, um zu speichern.",
        it: "Scegli un modello di OpenCode per salvare.",
        ja: "保存するには、OpenCodeのモデルを選んでください。",
        zh: "请选择 OpenCode 模型后再保存。",
        ko: "저장하려면 OpenCode 모델을 고르세요.",
        ru: "Выберите модель OpenCode, чтобы сохранить.",
    }
    /// Hint: the provider needs an address and a model before saving.
    ai_hint_fill_provider {
        en: "Enter the provider address and model to save.",
        pt: "Informe o endereço e o modelo do provedor para salvar.",
        es: "Indica la dirección y el modelo del proveedor para guardar.",
        fr: "Indiquez l’adresse et le modèle du fournisseur pour enregistrer.",
        de: "Gib die Adresse und das Modell des Anbieters an, um zu speichern.",
        it: "Indica l’indirizzo e il modello del provider per salvare.",
        ja: "保存するには、プロバイダーのアドレスとモデルを入力してください。",
        zh: "请填写提供商地址和模型后再保存。",
        ko: "저장하려면 공급자 주소와 모델을 입력하세요.",
        ru: "Укажите адрес и модель провайдера, чтобы сохранить.",
    }
    /// Hint: there are unsaved changes.
    ai_hint_unsaved {
        en: "Unsaved changes.",
        pt: "Alterações não salvas.",
        es: "Cambios sin guardar.",
        fr: "Modifications non enregistrées.",
        de: "Nicht gespeicherte Änderungen.",
        it: "Modifiche non salvate.",
        ja: "未保存の変更があります。",
        zh: "有未保存的更改。",
        ko: "저장하지 않은 변경 사항이 있어요.",
        ru: "Есть несохранённые изменения.",
    }
    /// Card and group label: who reads the captures.
    ai_extractor_title {
        en: "Extractor",
        pt: "Extrator",
        es: "Extractor",
        fr: "Extracteur",
        de: "Extraktor",
        it: "Estrattore",
        ja: "抽出器",
        zh: "提取器",
        ko: "추출기",
        ru: "Экстрактор",
    }
    /// Subtitle of the extractor card.
    ai_extractor_body {
        en: "Who reads the captures to propose decision candidates.",
        pt: "Quem lê as capturas para propor candidatos a decisão.",
        es: "Quién lee las capturas para proponer candidatos a decisión.",
        fr: "Qui lit les captures pour proposer des candidats de décision.",
        de: "Wer die Erfassungen liest, um Entscheidungskandidaten vorzuschlagen.",
        it: "Chi legge le acquisizioni per proporre candidati a decisione.",
        ja: "キャプチャを読み取り、決定の候補を提案するもの。",
        zh: "由谁读取捕获并提出决策候选。",
        ko: "캡처를 읽고 결정 후보를 제안하는 쪽이에요.",
        ru: "Кто читает захваты, чтобы предлагать кандидатов в решения.",
    }
    /// Preview value when something is not configured.
    ai_not_set {
        en: "Not set",
        pt: "Não configurado",
        es: "Sin configurar",
        fr: "Non configuré",
        de: "Nicht eingerichtet",
        it: "Non configurato",
        ja: "未設定",
        zh: "未配置",
        ko: "설정 안 됨",
        ru: "Не настроено",
    }
    /// Preview label: where the content goes.
    ai_preview_destination {
        en: "Destination",
        pt: "Destino",
        es: "Destino",
        fr: "Destination",
        de: "Ziel",
        it: "Destinazione",
        ja: "送信先",
        zh: "目的地",
        ko: "전송 대상",
        ru: "Назначение",
    }
    /// Preview label: how much goes in each analysis.
    ai_preview_per_analysis {
        en: "Per analysis",
        pt: "Por análise",
        es: "Por análisis",
        fr: "Par analyse",
        de: "Pro Analyse",
        it: "Per analisi",
        ja: "分析ごと",
        zh: "每次分析",
        ko: "분석당",
        ru: "На один анализ",
    }
    /// Preview card title.
    ai_preview_title {
        en: "What leaves the machine",
        pt: "O que sai da máquina",
        es: "Lo que sale de la máquina",
        fr: "Ce qui quitte la machine",
        de: "Was den Rechner verlässt",
        it: "Che cosa esce dalla macchina",
        ja: "マシンの外に出るもの",
        zh: "离开设备的内容",
        ko: "컴퓨터 밖으로 나가는 것",
        ru: "Что покидает компьютер",
    }
    /// Preview card subtitle.
    ai_preview_body {
        en: "An exact preview of what the provider may receive. Consent is tied to it.",
        pt: "Prévia exata do que o provedor pode receber. O consentimento fica ligado a ela.",
        es: "Vista previa exacta de lo que el proveedor puede recibir. El consentimiento queda \
            ligado a ella.",
        fr: "Aperçu exact de ce que le fournisseur peut recevoir. Le consentement y est lié.",
        de: "Genaue Vorschau dessen, was der Anbieter erhalten darf. Die Einwilligung ist daran \
            gebunden.",
        it: "Anteprima esatta di ciò che il provider può ricevere. Il consenso è legato ad essa.",
        ja: "プロバイダーが受け取る可能性のある内容そのままのプレビュー。同意はこのプレビューに紐づきます。",
        zh: "提供商可能收到内容的精确预览。同意与它绑定。",
        ko: "공급자가 받을 수 있는 내용을 그대로 보여주는 미리 보기예요. 동의는 이 미리 보기에 묶여 있어요.",
        ru: "Точный предпросмотр того, что может получить провайдер. Согласие привязано к нему.",
    }
    /// Preview category: the user's messages.
    ai_cat_user_text {
        en: "Messages you wrote",
        pt: "Mensagens que você escreveu",
        es: "Mensajes que escribiste",
        fr: "Messages que vous avez écrits",
        de: "Nachrichten, die du geschrieben hast",
        it: "Messaggi che hai scritto",
        ja: "あなたが書いたメッセージ",
        zh: "你写的消息",
        ko: "내가 쓴 메시지",
        ru: "Сообщения, которые вы написали",
    }
    /// Preview category: the assistant's replies.
    ai_cat_assistant_text {
        en: "Assistant replies",
        pt: "Respostas do assistente",
        es: "Respuestas del asistente",
        fr: "Réponses de l’assistant",
        de: "Antworten des Assistenten",
        it: "Risposte dell’assistente",
        ja: "アシスタントの返答",
        zh: "助手的回复",
        ko: "어시스턴트의 답변",
        ru: "Ответы ассистента",
    }
    /// Preview category: diff hunks.
    ai_cat_diff_hunk {
        en: "Diff hunks",
        pt: "Trechos de diff",
        es: "Fragmentos de diff",
        fr: "Extraits de diff",
        de: "Diff-Abschnitte",
        it: "Porzioni di diff",
        ja: "差分の抜粋",
        zh: "差异片段",
        ko: "diff 일부",
        ru: "Фрагменты диффа",
    }
    /// Preview category: tool summaries.
    ai_cat_tool_summary {
        en: "Tool summaries",
        pt: "Resumos de ferramentas",
        es: "Resúmenes de herramientas",
        fr: "Résumés d’outils",
        de: "Zusammenfassungen von Werkzeugen",
        it: "Riepiloghi degli strumenti",
        ja: "ツールの要約",
        zh: "工具摘要",
        ko: "도구 요약",
        ru: "Сводки по инструментам",
    }
    /// Preview category: project documents.
    ai_cat_document {
        en: "Project documents",
        pt: "Documentos do projeto",
        es: "Documentos del proyecto",
        fr: "Documents du projet",
        de: "Projektdokumente",
        it: "Documenti del progetto",
        ja: "プロジェクトのドキュメント",
        zh: "项目文档",
        ko: "프로젝트 문서",
        ru: "Документы проекта",
    }
    /// Preview category: document paths and metadata.
    ai_cat_document_metadata {
        en: "Document paths and metadata",
        pt: "Caminhos e metadados dos documentos",
        es: "Rutas y metadatos de los documentos",
        fr: "Chemins et métadonnées des documents",
        de: "Pfade und Metadaten der Dokumente",
        it: "Percorsi e metadati dei documenti",
        ja: "ドキュメントのパスとメタデータ",
        zh: "文档路径和元数据",
        ko: "문서 경로와 메타데이터",
        ru: "Пути и метаданные документов",
    }
    /// Preview category: decision fields.
    ai_cat_decision_fields {
        en: "Decision fields, scope and qualifiers",
        pt: "Campos, escopo e qualificadores das decisões",
        es: "Campos, alcance y calificadores de las decisiones",
        fr: "Champs, portée et qualificatifs des décisions",
        de: "Felder, Geltungsbereich und Qualifizierer der Entscheidungen",
        it: "Campi, ambito e qualificatori delle decisioni",
        ja: "決定のフィールド、範囲、修飾子",
        zh: "决策的字段、范围和限定词",
        ko: "결정의 필드, 범위, 한정어",
        ru: "Поля, область и уточнения решений",
    }
    /// Preview category: examples confirmed in review.
    ai_cat_confirmed_examples {
        en: "Examples confirmed in review",
        pt: "Exemplos confirmados na revisão",
        es: "Ejemplos confirmados en la revisión",
        fr: "Exemples confirmés en revue",
        de: "In der Prüfung bestätigte Beispiele",
        it: "Esempi confermati nella revisione",
        ja: "レビューで確定した例",
        zh: "审阅中已确认的示例",
        ko: "검토에서 확인한 예시",
        ru: "Примеры, подтверждённые при проверке",
    }
    /// Preview category: examples rejected in review.
    ai_cat_rejected_examples {
        en: "Examples rejected in review",
        pt: "Exemplos rejeitados na revisão",
        es: "Ejemplos rechazados en la revisión",
        fr: "Exemples rejetés en revue",
        de: "In der Prüfung abgelehnte Beispiele",
        it: "Esempi rifiutati nella revisione",
        ja: "レビューで却下した例",
        zh: "审阅中已拒绝的示例",
        ko: "검토에서 거부한 예시",
        ru: "Примеры, отклонённые при проверке",
    }
    /// Preview category: rules.
    ai_cat_rules {
        en: "Rules and their qualifiers",
        pt: "Regras e seus qualificadores",
        es: "Reglas y sus calificadores",
        fr: "Règles et leurs qualificatifs",
        de: "Regeln und ihre Qualifizierer",
        it: "Regole e relativi qualificatori",
        ja: "ルールとその修飾子",
        zh: "规则及其限定词",
        ko: "규칙과 한정어",
        ru: "Правила и их уточнения",
    }
    /// Preview category: project map items and links.
    ai_cat_project_map {
        en: "Project map items and links",
        pt: "Itens e vínculos do mapa do projeto",
        es: "Elementos y vínculos del mapa del proyecto",
        fr: "Éléments et liens de la carte du projet",
        de: "Elemente und Verknüpfungen der Projektkarte",
        it: "Elementi e collegamenti della mappa del progetto",
        ja: "プロジェクトのマップの項目とリンク",
        zh: "项目地图中的条目和关联",
        ko: "프로젝트 맵의 항목과 연결",
        ru: "Элементы и ссылки карты проекта",
    }
    /// Preview category: knowledge review sources and findings.
    ai_cat_knowledge_review {
        en: "Sources and findings of the knowledge review",
        pt: "Fontes e achados da revisão de conhecimento",
        es: "Fuentes y hallazgos de la revisión de conocimiento",
        fr: "Sources et constats de la revue des connaissances",
        de: "Quellen und Befunde der Wissensprüfung",
        it: "Fonti e riscontri della revisione della conoscenza",
        ja: "ナレッジレビューのソースと発見事項",
        zh: "知识审阅的来源和发现",
        ko: "지식 검토의 출처와 발견 사항",
        ru: "Источники и находки проверки знаний",
    }
    /// Preview category: task, files and memory selected to judge relevance.
    ai_cat_context_routing {
        en: "Task, files and memory selected to judge relevance",
        pt: "Tarefa, arquivos e memória selecionada para avaliar relevância",
        es: "Tarea, archivos y memoria seleccionada para evaluar la relevancia",
        fr: "Tâche, fichiers et mémoire sélectionnée pour évaluer la pertinence",
        de: "Aufgabe, Dateien und ausgewählter Speicher zur Beurteilung der Relevanz",
        it: "Attività, file e memoria selezionata per valutare la rilevanza",
        ja: "関連性を判断するために選んだタスク、ファイル、メモリ",
        zh: "为判断相关性而选出的任务、文件和记忆",
        ko: "관련성을 판단하려고 고른 작업, 파일, 메모리",
        ru: "Задача, файлы и выбранная память для оценки релевантности",
    }
    /// Preview category: anything else.
    ai_cat_other {
        en: "Other content",
        pt: "Outro conteúdo",
        es: "Otro contenido",
        fr: "Autre contenu",
        de: "Weitere Inhalte",
        it: "Altri contenuti",
        ja: "その他の内容",
        zh: "其他内容",
        ko: "기타 내용",
        ru: "Прочее содержимое",
    }
    /// Consent button: consent and turn on.
    ai_grant {
        en: "Consent and turn on",
        pt: "Consentir e ativar",
        es: "Dar consentimiento y activar",
        fr: "Consentir et activer",
        de: "Einwilligen und aktivieren",
        it: "Acconsenti e attiva",
        ja: "同意して有効にする",
        zh: "同意并启用",
        ko: "동의하고 켜기",
        ru: "Согласиться и включить",
    }
    /// Button: revoke consent.
    ai_revoke_consent {
        en: "Revoke consent",
        pt: "Revogar consentimento",
        es: "Revocar consentimiento",
        fr: "Révoquer le consentement",
        de: "Einwilligung widerrufen",
        it: "Revoca il consenso",
        ja: "同意を取り消す",
        zh: "撤销同意",
        ko: "동의 철회",
        ru: "Отозвать согласие",
    }
    /// Button: delete the key from the system credential store.
    ai_delete_key {
        en: "Delete key from the credential store",
        pt: "Apagar chave do cofre",
        es: "Borrar clave del almacén de credenciales",
        fr: "Supprimer la clé du coffre",
        de: "Schlüssel aus dem Speicher löschen",
        it: "Elimina la chiave dall’archivio",
        ja: "キーを資格情報ストアから削除",
        zh: "从凭据存储中删除密钥",
        ko: "자격 증명 저장소에서 키 삭제",
        ru: "Удалить ключ из хранилища",
    }
    /// Confirm button: turn off and delete the key.
    ai_revoke_and_delete {
        en: "Turn off and delete key",
        pt: "Desligar e apagar chave",
        es: "Desactivar y borrar clave",
        fr: "Désactiver et supprimer la clé",
        de: "Abschalten und Schlüssel löschen",
        it: "Disattiva ed elimina la chiave",
        ja: "オフにしてキーを削除",
        zh: "关闭并删除密钥",
        ko: "끄고 키 삭제",
        ru: "Отключить и удалить ключ",
    }
    /// Confirm button: turn off external calls.
    ai_revoke_calls {
        en: "Turn off external calls",
        pt: "Desligar chamadas externas",
        es: "Desactivar llamadas externas",
        fr: "Désactiver les appels externes",
        de: "Externe Aufrufe abschalten",
        it: "Disattiva le chiamate esterne",
        ja: "外部への呼び出しをオフにする",
        zh: "关闭外部调用",
        ko: "외부 호출 끄기",
        ru: "Отключить внешние вызовы",
    }
    /// Consent card title.
    ai_consent_title {
        en: "Consent",
        pt: "Consentimento",
        es: "Consentimiento",
        fr: "Consentement",
        de: "Einwilligung",
        it: "Consenso",
        ja: "同意",
        zh: "同意",
        ko: "동의",
        ru: "Согласие",
    }
    /// Consent card subtitle.
    ai_consent_body {
        en: "Nothing is sent before this step, and you can revoke it at any time.",
        pt: "Nada é enviado antes deste passo, e você pode revogar a qualquer momento.",
        es: "No se envía nada antes de este paso, y puedes revocarlo en cualquier momento.",
        fr: "Rien n’est envoyé avant cette étape, et vous pouvez la révoquer à tout moment.",
        de: "Vor diesem Schritt wird nichts gesendet, und du kannst ihn jederzeit widerrufen.",
        it: "Nulla viene inviato prima di questo passaggio, e puoi revocarlo in qualsiasi momento.",
        ja: "この手順の前には何も送信されず、いつでも取り消せます。",
        zh: "在这一步之前不会发送任何内容，你也可以随时撤销。",
        ko: "이 단계 전에는 아무것도 전송되지 않으며, 언제든 철회할 수 있어요.",
        ru: "До этого шага ничего не отправляется, и вы можете отозвать согласие в любой момент.",
    }
    /// Title of the load error of the AI section.
    ai_load_failed {
        en: "Couldn’t load the settings",
        pt: "Não foi possível carregar as configurações",
        es: "No se pudieron cargar los ajustes",
        fr: "Impossible de charger les réglages",
        de: "Die Einstellungen konnten nicht geladen werden",
        it: "Impossibile caricare le impostazioni",
        ja: "設定を読み込めませんでした",
        zh: "无法加载设置",
        ko: "설정을 불러올 수 없어요",
        ru: "Не удалось загрузить настройки",
    }
    /// Screen reader state of a checklist step that is met.
    step_done {
        en: "done",
        pt: "concluído",
        es: "hecho",
        fr: "terminé",
        de: "erledigt",
        it: "completato",
        ja: "完了",
        zh: "已完成",
        ko: "완료",
        ru: "выполнено",
    }
    /// Screen reader state of a checklist step that is still pending.
    step_pending {
        en: "pending",
        pt: "pendente",
        es: "pendiente",
        fr: "en attente",
        de: "ausstehend",
        it: "in sospeso",
        ja: "未完了",
        zh: "待完成",
        ko: "대기 중",
        ru: "ожидает",
    }
    /// Appearance: card title of the theme list.
    appearance_theme_title {
        en: "Theme",
        pt: "Tema",
        es: "Tema",
        fr: "Thème",
        de: "Design",
        it: "Tema",
        ja: "テーマ",
        zh: "主题",
        ko: "테마",
        ru: "Тема",
    }
    /// Appearance: card subtitle of the theme list.
    appearance_theme_body {
        en: "Colors across the whole app. Changes right away and is saved for next time.",
        pt: "Cores de todo o app. Muda na hora e fica salvo para a próxima vez.",
        es: "Colores de toda la app. Cambia al instante y se guarda para la próxima vez.",
        fr: "Les couleurs de toute l’app. S’applique tout de suite et reste enregistré.",
        de: "Farben der gesamten App. Gilt sofort und bleibt gespeichert.",
        it: "I colori di tutta l’app. Cambiano subito e restano salvati per la prossima volta.",
        ja: "アプリ全体の色。すぐに切り替わり、次回も保持されます。",
        zh: "整个应用的配色。立即生效，并保存到下次使用。",
        ko: "앱 전체의 색상이에요. 즉시 바뀌고 다음에도 유지돼요.",
        ru: "Цвета всего приложения. Меняются сразу и сохраняются на следующий раз.",
    }
    /// Appearance: card title of the background picker.
    appearance_background_title {
        en: "Background",
        pt: "Fundo",
        es: "Fondo",
        fr: "Arrière-plan",
        de: "Hintergrund",
        it: "Sfondo",
        ja: "背景",
        zh: "背景",
        ko: "배경",
        ru: "Фон",
    }
    /// Appearance: card subtitle of the background picker.
    appearance_background_body {
        en: "An image behind the surfaces, blurred and darkened so the text stays readable. The \
            app’s backgrounds are inspired by the Organization.",
        pt: "Uma imagem atrás das superfícies, desfocada e escurecida para o texto seguir \
            legível. Os fundos do app são inspirados na Organização.",
        es: "Una imagen detrás de las superficies, desenfocada y oscurecida para que el texto \
            siga siendo legible. Los fondos de la app se inspiran en la Organización.",
        fr: "Une image derrière les surfaces, floutée et assombrie pour que le texte reste \
            lisible. Les arrière-plans de l’app s’inspirent de l’Organisation.",
        de: "Ein Bild hinter den Flächen, weichgezeichnet und abgedunkelt, damit der Text \
            lesbar bleibt. Die Hintergründe der App sind von der Organisation inspiriert.",
        it: "Un’immagine dietro le superfici, sfocata e scurita perché il testo resti \
            leggibile. Gli sfondi dell’app si ispirano all’Organizzazione.",
        ja: "面の後ろに表示する画像です。文字が読みやすいように、ぼかして暗くしています。アプリの背景は「機関」にインスパイアされています。",
        zh: "界面后方的一张图片，经过模糊和压暗处理，让文字依然清晰易读。应用自带的背景灵感来自“组织”。",
        ko: "화면 뒤에 깔리는 이미지예요. 글자가 잘 보이도록 흐리게 하고 어둡게 처리해요. 앱 배경은 ‘기관’에서 영감을 받았어요.",
        ru: "Изображение за поверхностями, размытое и затемнённое, чтобы текст оставался \
            читаемым. Фоны приложения вдохновлены «Организацией».",
    }
    /// Appearance: tile name for no background.
    appearance_no_background {
        en: "No background",
        pt: "Sem fundo",
        es: "Sin fondo",
        fr: "Sans arrière-plan",
        de: "Kein Hintergrund",
        it: "Nessuno sfondo",
        ja: "背景なし",
        zh: "无背景",
        ko: "배경 없음",
        ru: "Без фона",
    }
    /// Appearance: caption inside the no-background tile.
    appearance_theme_only {
        en: "Theme only",
        pt: "Só o tema",
        es: "Solo el tema",
        fr: "Thème seul",
        de: "Nur das Design",
        it: "Solo il tema",
        ja: "テーマのみ",
        zh: "仅主题",
        ko: "테마만",
        ru: "Только тема",
    }
    /// Appearance: caption inside the empty custom-image tile.
    appearance_choose_image {
        en: "Choose image",
        pt: "Escolher imagem",
        es: "Elegir imagen",
        fr: "Choisir une image",
        de: "Bild auswählen",
        it: "Scegli immagine",
        ja: "画像を選ぶ",
        zh: "选择图片",
        ko: "이미지 선택",
        ru: "Выбрать изображение",
    }
    /// Appearance: tile name of the empty custom-image slot.
    appearance_your_image {
        en: "Your image",
        pt: "Sua imagem",
        es: "Tu imagen",
        fr: "Votre image",
        de: "Dein Bild",
        it: "La tua immagine",
        ja: "あなたの画像",
        zh: "你的图片",
        ko: "내 이미지",
        ru: "Ваше изображение",
    }
    /// Appearance: the chosen image could not be opened.
    appearance_image_failed {
        en: "Couldn’t open that image. Choose another one or go back to one of the app’s \
            backgrounds.",
        pt: "Não foi possível abrir essa imagem. Escolha outra ou volte a um fundo do app.",
        es: "No se pudo abrir esa imagen. Elige otra o vuelve a un fondo de la app.",
        fr: "Impossible d’ouvrir cette image. Choisissez-en une autre ou revenez à un \
            arrière-plan de l’app.",
        de: "Das Bild konnte nicht geöffnet werden. Wähle ein anderes oder kehre zu einem \
            Hintergrund der App zurück.",
        it: "Impossibile aprire questa immagine. Scegline un’altra o torna a uno sfondo dell’app.",
        ja: "この画像を開けませんでした。別の画像を選ぶか、アプリの背景に戻してください。",
        zh: "无法打开这张图片。请选择另一张，或改回应用自带的背景。",
        ko: "이 이미지를 열 수 없어요. 다른 이미지를 고르거나 앱 배경으로 되돌리세요.",
        ru: "Не удалось открыть это изображение. Выберите другое или вернитесь к фону приложения.",
    }
    /// Appearance: label of the background blur level.
    appearance_blur {
        en: "Blur",
        pt: "Desfoque",
        es: "Desenfoque",
        fr: "Flou",
        de: "Weichzeichnung",
        it: "Sfocatura",
        ja: "ぼかし",
        zh: "模糊",
        ko: "흐림",
        ru: "Размытие",
    }
    /// Appearance: label of the background darkening level.
    appearance_dim {
        en: "Darken",
        pt: "Escurecer",
        es: "Oscurecer",
        fr: "Assombrir",
        de: "Abdunkeln",
        it: "Scurisci",
        ja: "暗さ",
        zh: "压暗",
        ko: "어둡게",
        ru: "Затемнение",
    }
    /// Appearance: label of the surface solidity level.
    appearance_surfaces {
        en: "Surfaces",
        pt: "Superfícies",
        es: "Superficies",
        fr: "Surfaces",
        de: "Flächen",
        it: "Superfici",
        ja: "面",
        zh: "界面",
        ko: "표면",
        ru: "Поверхности",
    }
    /// Appearance: blur level, lowest.
    appearance_level_light {
        en: "Light",
        pt: "Leve",
        es: "Ligero",
        fr: "Léger",
        de: "Leicht",
        it: "Leggera",
        ja: "弱",
        zh: "轻",
        ko: "약하게",
        ru: "Слабое",
    }
    /// Appearance: middle level of blur and darkening.
    appearance_level_medium {
        en: "Medium",
        pt: "Médio",
        es: "Medio",
        fr: "Moyen",
        de: "Mittel",
        it: "Media",
        ja: "中",
        zh: "中",
        ko: "보통",
        ru: "Среднее",
    }
    /// Appearance: blur level, highest.
    appearance_level_strong {
        en: "Strong",
        pt: "Forte",
        es: "Fuerte",
        fr: "Fort",
        de: "Stark",
        it: "Forte",
        ja: "強",
        zh: "强",
        ko: "강하게",
        ru: "Сильное",
    }
    /// Appearance: darkening level, lowest.
    appearance_dim_little {
        en: "A little",
        pt: "Pouco",
        es: "Poco",
        fr: "Un peu",
        de: "Wenig",
        it: "Poco",
        ja: "少し",
        zh: "稍微",
        ko: "조금",
        ru: "Немного",
    }
    /// Appearance: darkening level, highest.
    appearance_dim_a_lot {
        en: "A lot",
        pt: "Bastante",
        es: "Bastante",
        fr: "Beaucoup",
        de: "Stark",
        it: "Molto",
        ja: "かなり",
        zh: "很多",
        ko: "많이",
        ru: "Сильно",
    }
    /// Appearance: surfaces level, most transparent.
    appearance_more_glass {
        en: "More glass",
        pt: "Mais vidro",
        es: "Más vidrio",
        fr: "Plus de verre",
        de: "Mehr Glas",
        it: "Più vetro",
        ja: "ガラス寄り",
        zh: "更通透",
        ko: "유리 느낌 더",
        ru: "Больше стекла",
    }
    /// Appearance: surfaces level, middle.
    appearance_balanced {
        en: "Balanced",
        pt: "Equilibradas",
        es: "Equilibradas",
        fr: "Équilibrées",
        de: "Ausgewogen",
        it: "Bilanciate",
        ja: "バランス",
        zh: "均衡",
        ko: "균형",
        ru: "Сбалансированные",
    }
    /// Appearance: surfaces level, most opaque.
    appearance_more_solid {
        en: "More solid",
        pt: "Mais sólidas",
        es: "Más sólidas",
        fr: "Plus pleines",
        de: "Mehr Deckkraft",
        it: "Più solide",
        ja: "不透明寄り",
        zh: "更实心",
        ko: "단단하게 더",
        ru: "Более плотные",
    }
    /// Appearance: title of the file dialog that picks a background.
    appearance_pick_title {
        en: "Choose a background image",
        pt: "Escolher imagem de fundo",
        es: "Elegir imagen de fondo",
        fr: "Choisir une image d’arrière-plan",
        de: "Hintergrundbild auswählen",
        it: "Scegli un’immagine di sfondo",
        ja: "背景画像を選ぶ",
        zh: "选择背景图片",
        ko: "배경 이미지 선택",
        ru: "Выбор фонового изображения",
    }
    /// Appearance: file dialog filter name for images.
    appearance_pick_filter {
        en: "Images",
        pt: "Imagens",
        es: "Imágenes",
        fr: "Images",
        de: "Bilder",
        it: "Immagini",
        ja: "画像",
        zh: "图片",
        ko: "이미지",
        ru: "Изображения",
    }
    /// Diagnostics error: the document could not be built.
    diag_err_document {
        en: "Couldn’t build the diagnostics.",
        pt: "Não foi possível montar o diagnóstico.",
        es: "No se pudo generar el diagnóstico.",
        fr: "Impossible de générer le diagnostic.",
        de: "Die Diagnose konnte nicht erstellt werden.",
        it: "Impossibile generare la diagnostica.",
        ja: "診断を作成できませんでした。",
        zh: "无法生成诊断。",
        ko: "진단을 만들 수 없어요.",
        ru: "Не удалось сформировать диагностику.",
    }
    /// Diagnostics error: a task could not be reprocessed.
    diag_err_reprocess {
        en: "Couldn’t reprocess the task; it may have already changed state.",
        pt: "Não foi possível reprocessar a tarefa; ela pode já ter mudado de estado.",
        es: "No se pudo reprocesar la tarea; puede que ya haya cambiado de estado.",
        fr: "Impossible de retraiter la tâche ; son état a peut-être déjà changé.",
        de: "Die Aufgabe konnte nicht neu verarbeitet werden; ihr Zustand hat sich \
            möglicherweise schon geändert.",
        it: "Impossibile rielaborare l’attività; potrebbe aver già cambiato stato.",
        ja: "タスクを再処理できませんでした。すでに状態が変わっている可能性があります。",
        zh: "无法重新处理该任务；它的状态可能已经改变。",
        ko: "작업을 재처리할 수 없어요. 이미 상태가 바뀌었을 수 있어요.",
        ru: "Не удалось обработать задачу заново: возможно, её состояние уже изменилось.",
    }
    /// Diagnostics error: a task could not be canceled.
    diag_err_cancel {
        en: "Couldn’t cancel the task; it may have already started.",
        pt: "Não foi possível cancelar a tarefa; ela pode já ter começado.",
        es: "No se pudo cancelar la tarea; puede que ya haya empezado.",
        fr: "Impossible d’annuler la tâche ; elle a peut-être déjà démarré.",
        de: "Die Aufgabe konnte nicht abgebrochen werden; sie hat möglicherweise schon begonnen.",
        it: "Impossibile annullare l’attività; potrebbe essere già iniziata.",
        ja: "タスクをキャンセルできませんでした。すでに開始されている可能性があります。",
        zh: "无法取消该任务；它可能已经开始。",
        ko: "작업을 취소할 수 없어요. 이미 시작되었을 수 있어요.",
        ru: "Не удалось отменить задачу: возможно, она уже началась.",
    }
    /// Diagnostics error: the queue counts could not be read.
    diag_err_lanes {
        en: "Couldn’t count the analysis queues.",
        pt: "Não foi possível contar as filas de análise.",
        es: "No se pudieron contar las colas de análisis.",
        fr: "Impossible de compter les files d’analyse.",
        de: "Die Analyse-Warteschlangen konnten nicht gezählt werden.",
        it: "Impossibile contare le code di analisi.",
        ja: "分析キューを集計できませんでした。",
        zh: "无法统计分析队列。",
        ko: "분석 대기열을 집계할 수 없어요.",
        ru: "Не удалось подсчитать очереди анализа.",
    }
    /// Diagnostics error: the parallel analyses setting could not be read.
    diag_err_parallel_read {
        en: "Couldn’t read the parallel analyses setting.",
        pt: "Não foi possível ler as análises em paralelo.",
        es: "No se pudo leer el número de análisis en paralelo.",
        fr: "Impossible de lire le nombre d’analyses en parallèle.",
        de: "Die Anzahl paralleler Analysen konnte nicht gelesen werden.",
        it: "Impossibile leggere il numero di analisi in parallelo.",
        ja: "並列分析の設定を読み込めませんでした。",
        zh: "无法读取并行分析设置。",
        ko: "병렬 분석 설정을 읽을 수 없어요.",
        ru: "Не удалось прочитать число параллельных анализов.",
    }
    /// Diagnostics error: the parallel analyses setting could not be saved.
    diag_err_parallel_save {
        en: "Couldn’t save the parallel analyses setting.",
        pt: "Não foi possível salvar as análises em paralelo.",
        es: "No se pudo guardar el número de análisis en paralelo.",
        fr: "Impossible d’enregistrer le nombre d’analyses en parallèle.",
        de: "Die Anzahl paralleler Analysen konnte nicht gespeichert werden.",
        it: "Impossibile salvare il numero di analisi in parallelo.",
        ja: "並列分析の設定を保存できませんでした。",
        zh: "无法保存并行分析设置。",
        ko: "병렬 분석 설정을 저장할 수 없어요.",
        ru: "Не удалось сохранить число параллельных анализов.",
    }
    /// Diagnostics toast: a capture went back to the queue.
    diag_capture_requeued {
        en: "Capture queued again for analysis.",
        pt: "Captura reenfileirada para análise.",
        es: "Captura puesta de nuevo en cola para análisis.",
        fr: "Capture remise en file d’attente pour analyse.",
        de: "Erfassung erneut zur Analyse eingereiht.",
        it: "Acquisizione rimessa in coda per l’analisi.",
        ja: "キャプチャを分析のために再度キューに入れました。",
        zh: "捕获已重新加入分析队列。",
        ko: "캡처를 분석 대기열에 다시 넣었어요.",
        ru: "Захват снова поставлен в очередь на анализ.",
    }
    /// Diagnostics error: the export file could not be generated.
    diag_err_file {
        en: "Couldn’t generate the file.",
        pt: "Não foi possível gerar o arquivo.",
        es: "No se pudo generar el archivo.",
        fr: "Impossible de générer le fichier.",
        de: "Die Datei konnte nicht erstellt werden.",
        it: "Impossibile generare il file.",
        ja: "ファイルを生成できませんでした。",
        zh: "无法生成文件。",
        ko: "파일을 만들 수 없어요.",
        ru: "Не удалось создать файл.",
    }
    /// Diagnostics error: the export file could not be written.
    diag_err_save_file {
        en: "Couldn’t save the file to that location.",
        pt: "Não foi possível salvar o arquivo nesse destino.",
        es: "No se pudo guardar el archivo en ese destino.",
        fr: "Impossible d’enregistrer le fichier à cet emplacement.",
        de: "Die Datei konnte an diesem Ort nicht gespeichert werden.",
        it: "Impossibile salvare il file in quella destinazione.",
        ja: "その場所にファイルを保存できませんでした。",
        zh: "无法将文件保存到该位置。",
        ko: "해당 위치에 파일을 저장할 수 없어요.",
        ru: "Не удалось сохранить файл в это место.",
    }
    /// Diagnostics: export card title, button label and file dialog title.
    diag_export_title {
        en: "Export diagnostics",
        pt: "Exportar diagnóstico",
        es: "Exportar diagnóstico",
        fr: "Exporter le diagnostic",
        de: "Diagnose exportieren",
        it: "Esporta diagnostica",
        ja: "診断をエクスポート",
        zh: "导出诊断",
        ko: "진단 내보내기",
        ru: "Экспорт диагностики",
    }
    /// Diagnostics: suggested file name of the export.
    diag_file_name {
        en: "xemnas-diagnostics.json",
        pt: "xemnas-diagnostico.json",
        es: "xemnas-diagnostico.json",
        fr: "xemnas-diagnostic.json",
        de: "xemnas-diagnose.json",
        it: "xemnas-diagnostica.json",
        ja: "xemnas-diagnostics.json",
        zh: "xemnas-diagnostics.json",
        ko: "xemnas-diagnostics.json",
        ru: "xemnas-diagnostics.json",
    }
    /// Diagnostics: tile label, time from capture to candidate.
    diag_stat_latency {
        en: "Capture → candidate",
        pt: "Captura → candidato",
        es: "Captura → candidato",
        fr: "Capture → candidat",
        de: "Erfassung → Kandidat",
        it: "Acquisizione → candidato",
        ja: "キャプチャ → 候補",
        zh: "捕获 → 候选",
        ko: "캡처 → 후보",
        ru: "Захват → кандидат",
    }
    /// Diagnostics: tile label, time until review.
    diag_stat_review_time {
        en: "Time to review",
        pt: "Tempo até revisão",
        es: "Tiempo hasta la revisión",
        fr: "Délai avant revue",
        de: "Zeit bis zur Prüfung",
        it: "Tempo alla revisione",
        ja: "レビューまでの時間",
        zh: "审阅等待时间",
        ko: "검토까지 시간",
        ru: "Время до проверки",
    }
    /// Diagnostics: tile label, share of dismissed candidates.
    diag_stat_dismissed {
        en: "Dismissed",
        pt: "Descartados",
        es: "Descartados",
        fr: "Écartés",
        de: "Verworfen",
        it: "Scartati",
        ja: "破棄",
        zh: "已丢弃",
        ko: "폐기",
        ru: "Отброшено",
    }
    /// Diagnostics: tile label, context blocks counted.
    diag_stat_context_blocks {
        en: "Context blocks",
        pt: "Blocos de contexto",
        es: "Bloques de contexto",
        fr: "Blocs de contexte",
        de: "Kontextblöcke",
        it: "Blocchi di contesto",
        ja: "コンテキストブロック",
        zh: "上下文块",
        ko: "컨텍스트 블록",
        ru: "Блоки контекста",
    }
    /// Diagnostics: note under the metrics tiles.
    diag_review_time_note {
        en: "Time to review includes waiting in the queue; it doesn’t measure the reviewer’s \
            active work.",
        pt: "Tempo até revisão inclui a espera na fila; não mede trabalho ativo do revisor.",
        es: "El tiempo hasta la revisión incluye la espera en la cola; no mide el trabajo \
            activo del revisor.",
        fr: "Le délai avant revue inclut l’attente dans la file ; il ne mesure pas le travail \
            actif du relecteur.",
        de: "Die Zeit bis zur Prüfung enthält die Wartezeit in der Warteschlange; die aktive \
            Arbeit des Prüfers wird nicht gemessen.",
        it: "Il tempo alla revisione include l’attesa in coda; non misura il lavoro attivo di \
            chi revisiona.",
        ja: "レビューまでの時間にはキューでの待ち時間が含まれ、レビュアーの実作業時間は測りません。",
        zh: "审阅等待时间包含排队等待，不衡量审阅者的实际工作时间。",
        ko: "검토까지 시간에는 대기열 대기 시간이 포함되며, 검토자의 실제 작업 시간은 측정하지 않아요.",
        ru: "Время до проверки включает ожидание в очереди и не измеряет активную работу \
            проверяющего.",
    }
    /// Diagnostics: verdict, confidence carries no signal.
    diag_calib_no_signal {
        en: "Confidence doesn’t separate what you accept from what you dismiss. Automatic \
            approval shouldn’t rely on it.",
        pt: "A confiança não separa o que você aceita do que descarta. Aprovação automática não \
            deve se apoiar nela.",
        es: "La confianza no separa lo que aceptas de lo que descartas. La aprobación \
            automática no debería apoyarse en ella.",
        fr: "La confiance ne distingue pas ce que vous acceptez de ce que vous écartez. \
            L’approbation automatique ne devrait pas s’y appuyer.",
        de: "Die Konfidenz trennt nicht, was du annimmst, von dem, was du verwirfst. \
            Automatische Freigabe sollte sich nicht darauf stützen.",
        it: "La confidenza non distingue ciò che accetti da ciò che scarti. L’approvazione \
            automatica non dovrebbe basarsi su di essa.",
        ja: "信頼度では、承認するものと破棄するものを区別できません。自動承認の根拠にすべきではありません。",
        zh: "置信度无法区分你接受和丢弃的内容，不应据此自动批准。",
        ko: "신뢰도로는 수락하는 것과 폐기하는 것을 가를 수 없어요. 자동 승인의 근거로 삼으면 안 돼요.",
        ru: "Уверенность не отделяет то, что вы принимаете, от того, что отбрасываете. \
            Автоодобрение не должно на неё опираться.",
    }
    /// Diagnostics: verdict, confidence is a weak signal.
    diag_calib_weak {
        en: "Confidence separates what you accept a little, but not enough to approve on its own.",
        pt: "A confiança separa um pouco o que você aceita, mas não o bastante para aprovar \
            sozinha.",
        es: "La confianza separa un poco lo que aceptas, pero no lo suficiente para aprobar por \
            sí sola.",
        fr: "La confiance distingue un peu ce que vous acceptez, mais pas assez pour approuver \
            seule.",
        de: "Die Konfidenz trennt das, was du annimmst, ein wenig, aber nicht genug, um allein \
            freizugeben.",
        it: "La confidenza distingue un po’ ciò che accetti, ma non abbastanza per approvare da \
            sola.",
        ja: "信頼度は承認するものをある程度区別できますが、単独で承認するには不十分です。",
        zh: "置信度能在一定程度上区分你接受的内容，但不足以单独用于批准。",
        ko: "신뢰도가 수락하는 것을 어느 정도 가르지만, 혼자서 승인하기에는 부족해요.",
        ru: "Уверенность немного отделяет то, что вы принимаете, но этого мало, чтобы одобрять \
            самостоятельно.",
    }
    /// Diagnostics: verdict, confidence predicts what is accepted.
    diag_calib_predicts {
        en: "Confidence predicts what you accept: automatic approval can rely on it.",
        pt: "A confiança prevê o que você aceita: dá para apoiar aprovação automática nela.",
        es: "La confianza predice lo que aceptas: se puede apoyar la aprobación automática en \
            ella.",
        fr: "La confiance prédit ce que vous acceptez : l’approbation automatique peut s’y \
            appuyer.",
        de: "Die Konfidenz sagt voraus, was du annimmst: Automatische Freigabe kann sich darauf \
            stützen.",
        it: "La confidenza prevede ciò che accetti: l’approvazione automatica può basarsi su di \
            essa.",
        ja: "信頼度は承認されるものを予測できるので、自動承認の根拠にできます。",
        zh: "置信度能预测你会接受什么，可以据此自动批准。",
        ko: "신뢰도가 수락할 것을 예측하므로 자동 승인의 근거로 삼을 수 있어요.",
        ru: "Уверенность предсказывает, что вы принимаете: автоодобрение может на неё опираться.",
    }
    /// Diagnostics: confidence calibration card title.
    diag_calib_title {
        en: "Is the extraction’s confidence any good?",
        pt: "A confiança da extração presta?",
        es: "¿Sirve la confianza de la extracción?",
        fr: "La confiance de l’extraction est-elle utile ?",
        de: "Taugt die Konfidenz der Extraktion?",
        it: "La confidenza dell’estrazione è affidabile?",
        ja: "抽出の信頼度は当てになる？",
        zh: "提取的置信度可靠吗？",
        ko: "추출 신뢰도는 쓸 만할까요?",
        ru: "Можно ли доверять уверенности извлечения?",
    }
    /// Diagnostics: confidence calibration card subtitle.
    diag_calib_body {
        en: "Among the candidates you’ve already decided on: how much of each confidence band \
            was accepted.",
        pt: "Entre os candidatos que você já decidiu: quanto da faixa de confiança foi aceito.",
        es: "Entre los candidatos que ya decidiste: cuánto de cada franja de confianza se aceptó.",
        fr: "Parmi les candidats que vous avez déjà tranchés : la part de chaque tranche de \
            confiance acceptée.",
        de: "Unter den Kandidaten, über die du schon entschieden hast: wie viel jeder \
            Konfidenzstufe angenommen wurde.",
        it: "Tra i candidati che hai già deciso: quanta parte di ogni fascia di confidenza è \
            stata accettata.",
        ja: "すでに決定した候補のうち、各信頼度の帯でどれだけ承認されたか。",
        zh: "在你已决定的候选中：各置信度区间有多少被接受。",
        ko: "이미 결정한 후보 중 신뢰도 구간별로 얼마나 수락했는지 보여줘요.",
        ru: "Среди кандидатов, по которым вы уже решили: какая доля каждого диапазона \
            уверенности принята.",
    }
    /// Diagnostics: losses card title.
    diag_losses_title {
        en: "Losses",
        pt: "Perdas",
        es: "Pérdidas",
        fr: "Pertes",
        de: "Verluste",
        it: "Perdite",
        ja: "損失",
        zh: "损失",
        ko: "손실",
        ru: "Потери",
    }
    /// Diagnostics: losses card subtitle.
    diag_losses_body {
        en: "Work that didn’t become a candidate. Zero everywhere is expected.",
        pt: "Trabalho que não virou candidato. Zero em tudo é o esperado.",
        es: "Trabajo que no llegó a ser candidato. Lo esperado es cero en todo.",
        fr: "Travail qui n’est pas devenu un candidat. Zéro partout est ce qui est attendu.",
        de: "Arbeit, aus der kein Kandidat wurde. Überall null ist der Normalfall.",
        it: "Lavoro che non è diventato un candidato. Zero ovunque è il risultato atteso.",
        ja: "候補にならなかった作業。すべて0であれば正常です。",
        zh: "未能成为候选的工作。全部为零才是预期状态。",
        ko: "후보가 되지 못한 작업이에요. 모두 0이면 정상이에요.",
        ru: "Работа, не ставшая кандидатом. Ноль везде — это норма.",
    }
    /// Diagnostics: loss row, failed analyses.
    diag_loss_assessments_failed {
        en: "Failed analyses",
        pt: "Análises que falharam",
        es: "Análisis fallidos",
        fr: "Analyses échouées",
        de: "Fehlgeschlagene Analysen",
        it: "Analisi non riuscite",
        ja: "失敗した分析",
        zh: "失败的分析",
        ko: "실패한 분석",
        ru: "Неудачные анализы",
    }
    /// Diagnostics: loss row, analyses skipped for lack of consent.
    diag_loss_assessments_skipped {
        en: "Analyses skipped for lack of consent",
        pt: "Análises puladas por falta de consentimento",
        es: "Análisis omitidos por falta de consentimiento",
        fr: "Analyses ignorées faute de consentement",
        de: "Wegen fehlender Einwilligung übersprungene Analysen",
        it: "Analisi saltate per mancanza di consenso",
        ja: "同意がないためスキップされた分析",
        zh: "因未同意而跳过的分析",
        ko: "동의가 없어 건너뛴 분석",
        ru: "Анализы, пропущенные из-за отсутствия согласия",
    }
    /// Diagnostics: loss row, failed tasks.
    diag_loss_jobs_failed {
        en: "Failed tasks",
        pt: "Tarefas que falharam",
        es: "Tareas fallidas",
        fr: "Tâches échouées",
        de: "Fehlgeschlagene Aufgaben",
        it: "Attività non riuscite",
        ja: "失敗したタスク",
        zh: "失败的任务",
        ko: "실패한 작업",
        ru: "Неудачные задачи",
    }
    /// Diagnostics: loss row, captures rejected in the outbox.
    diag_loss_outbox_rejected {
        en: "Captures rejected in the outbox",
        pt: "Capturas rejeitadas na outbox",
        es: "Capturas rechazadas en la outbox",
        fr: "Captures rejetées dans l’outbox",
        de: "In der Outbox abgelehnte Erfassungen",
        it: "Acquisizioni rifiutate nella outbox",
        ja: "アウトボックスで却下されたキャプチャ",
        zh: "发件箱中被拒绝的捕获",
        ko: "아웃박스에서 거부된 캡처",
        ru: "Захваты, отклонённые в outbox",
    }
    /// Diagnostics: label of the parallel analyses setting.
    diag_parallel_title {
        en: "Parallel analyses",
        pt: "Análises em paralelo",
        es: "Análisis en paralelo",
        fr: "Analyses en parallèle",
        de: "Parallele Analysen",
        it: "Analisi in parallelo",
        ja: "並列分析",
        zh: "并行分析",
        ko: "병렬 분석",
        ru: "Параллельные анализы",
    }
    /// Diagnostics toast: the parallel analyses setting was saved.
    diag_parallel_saved {
        en: "Saved. Takes effect the next time the app starts.",
        pt: "Salvo. Vale no próximo início do app.",
        es: "Guardado. Se aplica en el próximo inicio de la app.",
        fr: "Enregistré. S’applique au prochain démarrage de l’app.",
        de: "Gespeichert. Gilt ab dem nächsten Start der App.",
        it: "Salvato. Vale dal prossimo avvio dell’app.",
        ja: "保存しました。次回のアプリ起動から有効です。",
        zh: "已保存，下次启动应用时生效。",
        ko: "저장했어요. 다음에 앱을 켤 때부터 적용돼요.",
        ru: "Сохранено. Вступит в силу при следующем запуске приложения.",
    }
    /// Diagnostics: help text of the parallel analyses setting.
    diag_parallel_hint {
        en: "Simultaneous calls to the AI provider. Takes effect the next time the app starts.",
        pt: "Chamadas ao provedor de IA ao mesmo tempo. Vale no próximo início do app.",
        es: "Llamadas simultáneas al proveedor de IA. Se aplica en el próximo inicio de la app.",
        fr: "Appels simultanés au fournisseur d’IA. S’applique au prochain démarrage de l’app.",
        de: "Gleichzeitige Aufrufe an den KI-Anbieter. Gilt ab dem nächsten Start der App.",
        it: "Chiamate simultanee al provider di IA. Vale dal prossimo avvio dell’app.",
        ja: "AIプロバイダーへの同時呼び出し数。次回のアプリ起動から有効です。",
        zh: "同时向 AI 提供商发出的调用数。下次启动应用时生效。",
        ko: "AI 공급자에 동시에 보내는 호출 수예요. 다음에 앱을 켤 때부터 적용돼요.",
        ru: "Одновременные вызовы к ИИ-провайдеру. Вступит в силу при следующем запуске \
            приложения.",
    }
    /// Diagnostics: queues card title.
    diag_lanes_title {
        en: "Analysis queues",
        pt: "Filas de análise",
        es: "Colas de análisis",
        fr: "Files d’analyse",
        de: "Analyse-Warteschlangen",
        it: "Code di analisi",
        ja: "分析キュー",
        zh: "分析队列",
        ko: "분석 대기열",
        ru: "Очереди анализа",
    }
    /// Diagnostics: queues card subtitle.
    diag_lanes_body {
        en: "Recent sessions, documentation and suggestions run in separate queues.",
        pt: "Sessões recentes, documentação e sugestões andam em filas separadas.",
        es: "Las sesiones recientes, la documentación y las sugerencias van en colas separadas.",
        fr: "Les sessions récentes, la documentation et les suggestions passent par des files \
            séparées.",
        de: "Aktuelle Sitzungen, Dokumentation und Vorschläge laufen in getrennten Warteschlangen.",
        it: "Sessioni recenti, documentazione e suggerimenti seguono code separate.",
        ja: "最近のセッション、ドキュメント、提案は別々のキューで処理されます。",
        zh: "最近会话、文档和建议分别使用独立的队列。",
        ko: "최근 세션, 문서, 제안은 각각 다른 대기열에서 처리돼요.",
        ru: "Недавние сессии, документация и предложения идут в отдельных очередях.",
    }
    /// Diagnostics: queue name, recent sessions.
    diag_lane_now {
        en: "Recent sessions",
        pt: "Sessões recentes",
        es: "Sesiones recientes",
        fr: "Sessions récentes",
        de: "Aktuelle Sitzungen",
        it: "Sessioni recenti",
        ja: "最近のセッション",
        zh: "最近会话",
        ko: "최근 세션",
        ru: "Недавние сессии",
    }
    /// Diagnostics: what the recent sessions queue holds.
    diag_lane_now_hint {
        en: "Captures from agent sessions; they go first.",
        pt: "Capturas das sessões de agentes; passam na frente.",
        es: "Capturas de las sesiones de agentes; pasan primero.",
        fr: "Captures des sessions d’agents ; elles passent en premier.",
        de: "Erfassungen aus Agent-Sitzungen; sie kommen zuerst.",
        it: "Acquisizioni delle sessioni degli agenti; passano per prime.",
        ja: "エージェントのセッションからのキャプチャ。優先して処理されます。",
        zh: "来自智能体会话的捕获；优先处理。",
        ko: "에이전트 세션에서 온 캡처예요. 먼저 처리돼요.",
        ru: "Захваты из сессий агентов; идут первыми.",
    }
    /// Diagnostics: queue name, documentation.
    diag_lane_documents {
        en: "Documentation",
        pt: "Documentação",
        es: "Documentación",
        fr: "Documentation",
        de: "Dokumentation",
        it: "Documentazione",
        ja: "ドキュメント",
        zh: "文档",
        ko: "문서",
        ru: "Документация",
    }
    /// Diagnostics: what the documentation queue holds.
    diag_lane_documents_hint {
        en: "Documents imported from projects.",
        pt: "Documentos importados dos projetos.",
        es: "Documentos importados de los proyectos.",
        fr: "Documents importés depuis les projets.",
        de: "Aus Projekten importierte Dokumente.",
        it: "Documenti importati dai progetti.",
        ja: "プロジェクトから取り込んだドキュメント。",
        zh: "从项目导入的文档。",
        ko: "프로젝트에서 가져온 문서예요.",
        ru: "Документы, импортированные из проектов.",
    }
    /// Diagnostics: queue name, suggestions.
    diag_lane_suggestions {
        en: "Suggestions",
        pt: "Sugestões",
        es: "Sugerencias",
        fr: "Suggestions",
        de: "Vorschläge",
        it: "Suggerimenti",
        ja: "提案",
        zh: "建议",
        ko: "제안",
        ru: "Предложения",
    }
    /// Diagnostics: what the suggestions queue holds.
    diag_lane_suggestions_hint {
        en: "Relations and rules suggested after adopting.",
        pt: "Relações e regras sugeridas após adotar.",
        es: "Relaciones y reglas sugeridas tras adoptar.",
        fr: "Relations et règles suggérées après adoption.",
        de: "Nach dem Übernehmen vorgeschlagene Beziehungen und Regeln.",
        it: "Relazioni e regole suggerite dopo l’adozione.",
        ja: "採用後に提案される関係とルール。",
        zh: "采用后建议的关系和规则。",
        ko: "채택한 뒤 제안되는 관계와 규칙이에요.",
        ru: "Связи и правила, предложенные после принятия.",
    }
    /// Diagnostics: no tasks recorded yet.
    diag_jobs_empty {
        en: "No tasks recorded yet. Every capture received creates an analysis.",
        pt: "Nenhuma tarefa registrada ainda. Cada captura recebida gera uma análise.",
        es: "Aún no hay tareas registradas. Cada captura recibida genera un análisis.",
        fr: "Aucune tâche enregistrée pour l’instant. Chaque capture reçue génère une analyse.",
        de: "Noch keine Aufgaben erfasst. Jede empfangene Erfassung erzeugt eine Analyse.",
        it: "Nessuna attività registrata finora. Ogni acquisizione ricevuta genera un’analisi.",
        ja: "まだタスクはありません。キャプチャを受け取るたびに分析が作られます。",
        zh: "还没有任务记录。每收到一个捕获就会生成一次分析。",
        ko: "아직 기록된 작업이 없어요. 캡처를 받을 때마다 분석이 만들어져요.",
        ru: "Задач пока нет. Каждый полученный захват создаёт анализ.",
    }
    /// Diagnostics toast: a task went back to the queue.
    diag_job_requeued {
        en: "Task back in the queue.",
        pt: "Tarefa de volta na fila.",
        es: "Tarea de vuelta en la cola.",
        fr: "Tâche remise dans la file.",
        de: "Aufgabe wieder in der Warteschlange.",
        it: "Attività di nuovo in coda.",
        ja: "タスクをキューに戻しました。",
        zh: "任务已重新排队。",
        ko: "작업을 대기열에 다시 넣었어요.",
        ru: "Задача снова в очереди.",
    }
    /// Diagnostics toast: a task was canceled.
    diag_job_cancelled {
        en: "Task canceled.",
        pt: "Tarefa cancelada.",
        es: "Tarea cancelada.",
        fr: "Tâche annulée.",
        de: "Aufgabe abgebrochen.",
        it: "Attività annullata.",
        ja: "タスクをキャンセルしました。",
        zh: "任务已取消。",
        ko: "작업을 취소했어요.",
        ru: "Задача отменена.",
    }
    /// Diagnostics: recent tasks card title.
    diag_jobs_title {
        en: "Recent tasks",
        pt: "Tarefas recentes",
        es: "Tareas recientes",
        fr: "Tâches récentes",
        de: "Aktuelle Aufgaben",
        it: "Attività recenti",
        ja: "最近のタスク",
        zh: "最近的任务",
        ko: "최근 작업",
        ru: "Недавние задачи",
    }
    /// Diagnostics: recent tasks card subtitle.
    diag_jobs_body {
        en: "The latest capture analyses. Failures can be reprocessed.",
        pt: "As últimas análises de captura. Falhas podem ser reprocessadas.",
        es: "Los últimos análisis de captura. Los fallos se pueden reprocesar.",
        fr: "Les dernières analyses de capture. Les échecs peuvent être retraités.",
        de: "Die letzten Erfassungsanalysen. Fehlgeschlagene können neu verarbeitet werden.",
        it: "Le ultime analisi delle acquisizioni. Quelle non riuscite si possono rielaborare.",
        ja: "最新のキャプチャ分析。失敗したものは再処理できます。",
        zh: "最近的捕获分析。失败的可以重新处理。",
        ko: "최근 캡처 분석이에요. 실패한 항목은 재처리할 수 있어요.",
        ru: "Последние анализы захватов. Неудачные можно обработать заново.",
    }
    /// Diagnostics: task state, queued.
    diag_state_queued {
        en: "Queued",
        pt: "Na fila",
        es: "En cola",
        fr: "En attente",
        de: "In Warteschlange",
        it: "In coda",
        ja: "待機中",
        zh: "排队中",
        ko: "대기 중",
        ru: "В очереди",
    }
    /// Diagnostics: task state, running.
    diag_state_running {
        en: "Running",
        pt: "Executando",
        es: "En ejecución",
        fr: "En cours",
        de: "Läuft",
        it: "In esecuzione",
        ja: "実行中",
        zh: "运行中",
        ko: "실행 중",
        ru: "Выполняется",
    }
    /// Diagnostics: task state, completed.
    diag_state_completed {
        en: "Completed",
        pt: "Concluída",
        es: "Completada",
        fr: "Terminée",
        de: "Abgeschlossen",
        it: "Completata",
        ja: "完了",
        zh: "已完成",
        ko: "완료",
        ru: "Завершена",
    }
    /// Diagnostics: task state, failed.
    diag_state_failed {
        en: "Failed",
        pt: "Falhou",
        es: "Falló",
        fr: "Échec",
        de: "Fehlgeschlagen",
        it: "Non riuscita",
        ja: "失敗",
        zh: "失败",
        ko: "실패",
        ru: "Ошибка",
    }
    /// Diagnostics: task state, canceled.
    diag_state_cancelled {
        en: "Canceled",
        pt: "Cancelada",
        es: "Cancelada",
        fr: "Annulée",
        de: "Abgebrochen",
        it: "Annullata",
        ja: "キャンセル済み",
        zh: "已取消",
        ko: "취소됨",
        ru: "Отменена",
    }
    /// Diagnostics: task state not recognised.
    diag_state_unknown {
        en: "Unknown",
        pt: "Desconhecido",
        es: "Desconocido",
        fr: "Inconnu",
        de: "Unbekannt",
        it: "Sconosciuto",
        ja: "不明",
        zh: "未知",
        ko: "알 수 없음",
        ru: "Неизвестно",
    }
    /// Diagnostics: task kind, capture analysis.
    diag_kind_capture {
        en: "Capture analysis",
        pt: "Análise de captura",
        es: "Análisis de captura",
        fr: "Analyse de capture",
        de: "Erfassungsanalyse",
        it: "Analisi di acquisizione",
        ja: "キャプチャ分析",
        zh: "捕获分析",
        ko: "캡처 분석",
        ru: "Анализ захвата",
    }
    /// Diagnostics: task kind, document analysis.
    diag_kind_document {
        en: "Document analysis",
        pt: "Análise de documento",
        es: "Análisis de documento",
        fr: "Analyse de document",
        de: "Dokumentanalyse",
        it: "Analisi di documento",
        ja: "ドキュメント分析",
        zh: "文档分析",
        ko: "문서 분석",
        ru: "Анализ документа",
    }
    /// Diagnostics: task kind, suggested relations.
    diag_kind_relations {
        en: "Suggested relations",
        pt: "Relações sugeridas",
        es: "Relaciones sugeridas",
        fr: "Relations suggérées",
        de: "Vorgeschlagene Beziehungen",
        it: "Relazioni suggerite",
        ja: "提案された関係",
        zh: "建议的关系",
        ko: "제안된 관계",
        ru: "Предложенные связи",
    }
    /// Diagnostics: task kind, suggested rules.
    diag_kind_rules {
        en: "Suggested rules",
        pt: "Regras sugeridas",
        es: "Reglas sugeridas",
        fr: "Règles suggérées",
        de: "Vorgeschlagene Regeln",
        it: "Regole suggerite",
        ja: "提案されたルール",
        zh: "建议的规则",
        ko: "제안된 규칙",
        ru: "Предложенные правила",
    }
    /// Diagnostics: label of the version row.
    diag_about_version {
        en: "Version",
        pt: "Versão",
        es: "Versión",
        fr: "Version",
        de: "Version",
        it: "Versione",
        ja: "バージョン",
        zh: "版本",
        ko: "버전",
        ru: "Версия",
    }
    /// Diagnostics: label of the data counts row.
    diag_about_data {
        en: "Data",
        pt: "Dados",
        es: "Datos",
        fr: "Données",
        de: "Daten",
        it: "Dati",
        ja: "データ",
        zh: "数据",
        ko: "데이터",
        ru: "Данные",
    }
    /// Diagnostics: label of the extraction row.
    diag_about_extraction {
        en: "Extraction",
        pt: "Extração",
        es: "Extracción",
        fr: "Extraction",
        de: "Extraktion",
        it: "Estrazione",
        ja: "抽出",
        zh: "提取",
        ko: "추출",
        ru: "Извлечение",
    }
    /// Diagnostics: export card subtitle.
    diag_export_body {
        en: "A JSON file with counts, metrics and error codes. No conversations, diffs, \
            decisions or credentials.",
        pt: "Um JSON com contagens, métricas e códigos de erro. Sem conversas, diffs, decisões \
            ou credenciais.",
        es: "Un JSON con recuentos, métricas y códigos de error. Sin conversaciones, diffs, \
            decisiones ni credenciales.",
        fr: "Un JSON avec des compteurs, des métriques et des codes d’erreur. Sans \
            conversations, diffs, décisions ni identifiants.",
        de: "Eine JSON-Datei mit Zählwerten, Metriken und Fehlercodes. Ohne Unterhaltungen, \
            Diffs, Entscheidungen oder Zugangsdaten.",
        it: "Un JSON con conteggi, metriche e codici di errore. Senza conversazioni, diff, \
            decisioni o credenziali.",
        ja: "件数、指標、エラーコードをまとめたJSON。会話、差分、決定、認証情報は含まれません。",
        zh: "包含计数、指标和错误代码的 JSON。不含对话、差异、决策或凭据。",
        ko: "개수, 지표, 오류 코드가 담긴 JSON이에요. 대화, diff, 결정, 자격 증명은 들어 있지 않아요.",
        ru: "JSON со счётчиками, метриками и кодами ошибок. Без разговоров, диффов, решений и \
            учётных данных.",
    }
    /// Diagnostics: advice next to the export button.
    diag_export_footer {
        en: "Attach it to a bug report; review the file before sending.",
        pt: "Anexe a um relato de problema; revise o arquivo antes de enviar.",
        es: "Adjúntalo a un informe de problema; revisa el archivo antes de enviarlo.",
        fr: "Joignez-le à un rapport de problème ; relisez le fichier avant de l’envoyer.",
        de: "Hänge sie an eine Fehlermeldung an; prüfe die Datei vor dem Senden.",
        it: "Allegalo a una segnalazione di problema; controlla il file prima di inviarlo.",
        ja: "不具合の報告に添付してください。送る前にファイルの内容を確認してください。",
        zh: "附在问题报告中；发送前请先检查文件。",
        ko: "문제 보고에 첨부하세요. 보내기 전에 파일을 확인하세요.",
        ru: "Приложите к сообщению о проблеме; перед отправкой проверьте файл.",
    }
    /// Diagnostics: export button label.
    diag_export_button {
        en: "Export…",
        pt: "Exportar…",
        es: "Exportar…",
        fr: "Exporter…",
        de: "Exportieren…",
        it: "Esporta…",
        ja: "エクスポート…",
        zh: "导出…",
        ko: "내보내기…",
        ru: "Экспорт…",
    }
    /// Diagnostics: section label of the selected project's captures.
    diag_captures_section {
        en: "Captures of the selected project",
        pt: "Capturas do projeto selecionado",
        es: "Capturas del proyecto seleccionado",
        fr: "Captures du projet sélectionné",
        de: "Erfassungen des ausgewählten Projekts",
        it: "Acquisizioni del progetto selezionato",
        ja: "選択中のプロジェクトのキャプチャ",
        zh: "所选项目的捕获",
        ko: "선택한 프로젝트의 캡처",
        ru: "Захваты выбранного проекта",
    }
    /// Diagnostics error: a capture could not be reprocessed.
    diag_err_reprocess_capture {
        en: "Couldn’t reprocess this capture.",
        pt: "Não foi possível reprocessar esta captura.",
        es: "No se pudo reprocesar esta captura.",
        fr: "Impossible de retraiter cette capture.",
        de: "Diese Erfassung konnte nicht neu verarbeitet werden.",
        it: "Impossibile rielaborare questa acquisizione.",
        ja: "このキャプチャを再処理できませんでした。",
        zh: "无法重新处理此捕获。",
        ko: "이 캡처를 재처리할 수 없어요.",
        ru: "Не удалось обработать этот захват заново.",
    }
    /// Diagnostics: screen reader label of the reprocess retry button.
    diag_retry_reprocess_label {
        en: "Try reprocessing again",
        pt: "Tentar reprocessar novamente",
        es: "Reintentar el reprocesamiento",
        fr: "Réessayer le retraitement",
        de: "Neu verarbeiten erneut versuchen",
        it: "Riprova la rielaborazione",
        ja: "再処理をもう一度試す",
        zh: "再次尝试重新处理",
        ko: "재처리 다시 시도",
        ru: "Повторить обработку заново",
    }
    /// Diagnostics: reprocess retry button.
    diag_retry_reprocess {
        en: "Try reprocessing",
        pt: "Tentar reprocessar",
        es: "Reintentar reprocesar",
        fr: "Réessayer de retraiter",
        de: "Erneut verarbeiten",
        it: "Riprova a rielaborare",
        ja: "再処理を再試行",
        zh: "重试重新处理",
        ko: "재처리 다시 시도",
        ru: "Повторить обработку",
    }
    /// Diagnostics: screen reader label of the button that closes the reprocess error.
    diag_dismiss_error_label {
        en: "Dismiss reprocessing error",
        pt: "Dispensar erro de reprocessamento",
        es: "Descartar el error de reprocesamiento",
        fr: "Ignorer l’erreur de retraitement",
        de: "Fehler bei der Neuverarbeitung ausblenden",
        it: "Ignora l’errore di rielaborazione",
        ja: "再処理のエラーを閉じる",
        zh: "关闭重新处理错误",
        ko: "재처리 오류 닫기",
        ru: "Скрыть ошибку обработки",
    }
    /// Diagnostics: button that closes the reprocess error.
    diag_dismiss {
        en: "Dismiss",
        pt: "Dispensar",
        es: "Descartar",
        fr: "Ignorer",
        de: "Ausblenden",
        it: "Ignora",
        ja: "閉じる",
        zh: "忽略",
        ko: "닫기",
        ru: "Скрыть",
    }
    /// Diagnostics error: the capture projection could not be refreshed.
    diag_err_captures {
        en: "Couldn’t refresh what happened to the captures.",
        pt: "Não foi possível atualizar o destino das capturas.",
        es: "No se pudo actualizar el destino de las capturas.",
        fr: "Impossible d’actualiser la destination des captures.",
        de: "Der Verbleib der Erfassungen konnte nicht aktualisiert werden.",
        it: "Impossibile aggiornare la destinazione delle acquisizioni.",
        ja: "キャプチャの処理状況を更新できませんでした。",
        zh: "无法更新捕获的去向。",
        ko: "캡처 진행 상황을 새로 고칠 수 없어요.",
        ru: "Не удалось обновить, куда попали захваты.",
    }
    /// Diagnostics: screen reader label of the capture refresh button.
    diag_refresh_captures_label {
        en: "Refresh captures",
        pt: "Atualizar capturas",
        es: "Actualizar capturas",
        fr: "Actualiser les captures",
        de: "Erfassungen aktualisieren",
        it: "Aggiorna acquisizioni",
        ja: "キャプチャを更新",
        zh: "刷新捕获",
        ko: "캡처 새로 고침",
        ru: "Обновить захваты",
    }
    /// Diagnostics: no project selected.
    diag_select_project {
        en: "Select a project to see its captures.",
        pt: "Selecione um projeto para consultar suas capturas.",
        es: "Selecciona un proyecto para ver sus capturas.",
        fr: "Sélectionnez un projet pour consulter ses captures.",
        de: "Wähle ein Projekt aus, um seine Erfassungen zu sehen.",
        it: "Seleziona un progetto per vedere le sue acquisizioni.",
        ja: "プロジェクトを選ぶと、そのキャプチャを確認できます。",
        zh: "选择一个项目以查看其捕获。",
        ko: "프로젝트를 선택하면 캡처를 볼 수 있어요.",
        ru: "Выберите проект, чтобы увидеть его захваты.",
    }
    /// Diagnostics: captures are loading.
    diag_loading_captures {
        en: "Loading captures…",
        pt: "Carregando capturas…",
        es: "Cargando capturas…",
        fr: "Chargement des captures…",
        de: "Erfassungen werden geladen…",
        it: "Caricamento acquisizioni…",
        ja: "キャプチャを読み込み中…",
        zh: "正在加载捕获…",
        ko: "캡처 불러오는 중…",
        ru: "Загрузка захватов…",
    }
    /// Diagnostics: captures could not be read.
    diag_captures_unavailable {
        en: "Captures unavailable.",
        pt: "Capturas indisponíveis.",
        es: "Capturas no disponibles.",
        fr: "Captures indisponibles.",
        de: "Erfassungen nicht verfügbar.",
        it: "Acquisizioni non disponibili.",
        ja: "キャプチャを表示できません。",
        zh: "捕获不可用。",
        ko: "캡처를 사용할 수 없어요.",
        ru: "Захваты недоступны.",
    }
    /// Diagnostics: the project has no captures.
    diag_no_captures {
        en: "No captures received in this project.",
        pt: "Nenhuma captura recebida neste projeto.",
        es: "Ninguna captura recibida en este proyecto.",
        fr: "Aucune capture reçue dans ce projet.",
        de: "In diesem Projekt wurden keine Erfassungen empfangen.",
        it: "Nessuna acquisizione ricevuta in questo progetto.",
        ja: "このプロジェクトで受け取ったキャプチャはありません。",
        zh: "此项目尚未收到捕获。",
        ko: "이 프로젝트에서 받은 캡처가 없어요.",
        ru: "В этом проекте захватов пока нет.",
    }
    /// Diagnostics: screen reader label of the capture reprocess button.
    diag_reprocess_capture_label {
        en: "Reprocess capture",
        pt: "Reprocessar captura",
        es: "Reprocesar captura",
        fr: "Retraiter la capture",
        de: "Erfassung neu verarbeiten",
        it: "Rielabora acquisizione",
        ja: "キャプチャを再処理",
        zh: "重新处理捕获",
        ko: "캡처 재처리",
        ru: "Обработать захват заново",
    }
    /// Provider form: description of the OpenCode Zen plan.
    plan_zen_body {
        en: "Pay as you go; includes free models.",
        pt: "Paga por uso; inclui modelos gratuitos.",
        es: "Pago por uso; incluye modelos gratuitos.",
        fr: "Paiement à l’usage ; inclut des modèles gratuits.",
        de: "Abrechnung nach Nutzung; enthält kostenlose Modelle.",
        it: "A consumo; include modelli gratuiti.",
        ja: "従量課金。無料のモデルも含まれます。",
        zh: "按用量付费；包含免费模型。",
        ko: "사용한 만큼 결제해요. 무료 모델도 포함돼요.",
        ru: "Оплата по факту использования; есть бесплатные модели.",
    }
    /// Provider form: description of the OpenCode Go plan.
    plan_go_body {
        en: "Monthly subscription to open models.",
        pt: "Assinatura mensal de modelos abertos.",
        es: "Suscripción mensual a modelos abiertos.",
        fr: "Abonnement mensuel à des modèles ouverts.",
        de: "Monatliches Abonnement für offene Modelle.",
        it: "Abbonamento mensile a modelli aperti.",
        ja: "オープンモデルの月額サブスクリプション。",
        zh: "开放模型的月度订阅。",
        ko: "오픈 모델을 월 구독으로 써요.",
        ru: "Ежемесячная подписка на открытые модели.",
    }
    /// ChatGPT account card: what signing in does.
    account_description {
        en: "Sign in with your account to extract using your ChatGPT plan, no API key. Only a \
            refresh token is kept in the system credential store.",
        pt: "Entre com a sua conta para extrair com o seu plano do ChatGPT, sem chave de API. \
            Só um token de renovação fica no cofre do sistema.",
        es: "Inicia sesión con tu cuenta para extraer con tu plan de ChatGPT, sin clave de API. \
            Solo un token de renovación se guarda en el almacén de credenciales del sistema.",
        fr: "Connectez-vous avec votre compte pour extraire avec votre forfait ChatGPT, sans \
            clé d’API. Seul un jeton d’actualisation est conservé dans le coffre d’identifiants \
            du système.",
        de: "Melde dich mit deinem Konto an, um mit deinem ChatGPT-Tarif zu extrahieren, ohne \
            API-Schlüssel. Nur ein Aktualisierungstoken wird im Anmeldeinformationsspeicher des \
            Systems abgelegt.",
        it: "Accedi con il tuo account per estrarre con il tuo piano ChatGPT, senza chiave API. \
            Solo un token di rinnovo resta nell’archivio credenziali di sistema.",
        ja: "アカウントでサインインすると、APIキーなしでChatGPTのプランを使って抽出できます。システムの資格情報ストアに残るのは更新トークンだけです。",
        zh: "用你的账户登录，即可使用 ChatGPT 套餐进行提取，无需 API 密钥。系统凭据存储中只保留一个刷新令牌。",
        ko: "계정으로 로그인하면 API 키 없이 ChatGPT 플랜으로 추출할 수 있어요. 시스템 자격 증명 저장소에는 갱신 토큰만 남아요.",
        ru: "Войдите в свою учётную запись, чтобы извлекать данные по тарифу ChatGPT без \
            API-ключа. В системном хранилище учётных данных остаётся только токен обновления.",
    }
    /// ChatGPT account card: the plan usage notice after the first sign-in.
    account_plan_notice {
        en: "xemnas analyses count toward your plan’s usage, like chats in ChatGPT. Track it in \
            Manage usage.",
        pt: "As análises da xemnas contam no uso do seu plano, como conversas no ChatGPT. \
            Acompanhe em Gerenciar uso.",
        es: "Los análisis de xemnas cuentan en el uso de tu plan, como las conversaciones en \
            ChatGPT. Síguelo en Gestionar uso.",
        fr: "Les analyses de xemnas comptent dans l’utilisation de votre forfait, comme les \
            conversations dans ChatGPT. Suivez-les dans Gérer l’utilisation.",
        de: "Die Analysen von xemnas zählen zur Nutzung deines Tarifs, wie Unterhaltungen in \
            ChatGPT. Verfolge sie unter Nutzung verwalten.",
        it: "Le analisi di xemnas contano nell’utilizzo del tuo piano, come le conversazioni in \
            ChatGPT. Seguile in Gestisci utilizzo.",
        ja: "xemnasの分析は、ChatGPTでの会話と同じくプランの使用量に含まれます。「使用状況を管理」で確認できます。",
        zh: "xemnas 的分析会计入你套餐的用量，就像在 ChatGPT 中聊天一样。可在“管理用量”中查看。",
        ko: "xemnas 분석은 ChatGPT 대화처럼 플랜 사용량에 포함돼요. ‘사용량 관리’에서 확인하세요.",
        ru: "Анализы xemnas учитываются в использовании вашего тарифа, как разговоры в ChatGPT. \
            Следите за этим в разделе «Управление использованием».",
    }
    /// ChatGPT account card: link to the plan usage page.
    account_manage_usage {
        en: "Manage usage",
        pt: "Gerenciar uso",
        es: "Gestionar uso",
        fr: "Gérer l’utilisation",
        de: "Nutzung verwalten",
        it: "Gestisci utilizzo",
        ja: "使用状況を管理",
        zh: "管理用量",
        ko: "사용량 관리",
        ru: "Управление использованием",
    }
    /// Key card: where the OpenCode key lives.
    key_opencode_description {
        en: "The same key works for Zen and Go. It stays in the system credential manager and \
            isn’t shown here.",
        pt: "A mesma chave serve para o Zen e o Go. Fica no Gerenciador de Credenciais do \
            sistema e não é exibida aqui.",
        es: "La misma clave sirve para Zen y Go. Se guarda en el administrador de credenciales \
            del sistema y no se muestra aquí.",
        fr: "La même clé sert pour Zen et Go. Elle reste dans le gestionnaire d’identifiants du \
            système et n’est pas affichée ici.",
        de: "Derselbe Schlüssel gilt für Zen und Go. Er liegt in der \
            Anmeldeinformationsverwaltung des Systems und wird hier nicht angezeigt.",
        it: "La stessa chiave vale per Zen e Go. Resta nella gestione credenziali di sistema e \
            non viene mostrata qui.",
        ja: "ZenとGoで同じキーを使えます。システムの資格情報マネージャーに保存され、ここには表示されません。",
        zh: "Zen 和 Go 共用同一个密钥。它保存在系统凭据管理器中，不会在此显示。",
        ko: "Zen과 Go에서 같은 키를 써요. 시스템 자격 증명 관리자에 저장되고 여기에는 표시되지 않아요.",
        ru: "Один и тот же ключ подходит для Zen и Go. Он хранится в системном менеджере \
            учётных данных и здесь не показывается.",
    }
    /// ChatGPT account card: waiting for the browser sign-in.
    account_waiting_browser {
        en: "Finish signing in in your browser. This screen updates by itself when you come back.",
        pt: "Conclua o login no navegador. Esta tela atualiza sozinha quando você voltar.",
        es: "Termina el inicio de sesión en el navegador. Esta pantalla se actualiza sola \
            cuando vuelvas.",
        fr: "Terminez la connexion dans le navigateur. Cet écran se met à jour tout seul à \
            votre retour.",
        de: "Schließe die Anmeldung im Browser ab. Diese Ansicht aktualisiert sich von selbst, \
            wenn du zurückkommst.",
        it: "Completa l’accesso nel browser. Questa schermata si aggiorna da sola quando torni.",
        ja: "ブラウザーでサインインを完了してください。戻ってくると、この画面は自動で更新されます。",
        zh: "请在浏览器中完成登录。你回来后，此界面会自动更新。",
        ko: "브라우저에서 로그인을 마치세요. 돌아오면 이 화면이 알아서 새로 고쳐져요.",
        ru: "Завершите вход в браузере. Когда вы вернётесь, эта страница обновится сама.",
    }
    /// Status line: captures are analyzed on this machine.
    analysed_here {
        en: "on this machine",
        pt: "nesta máquina",
        es: "en esta máquina",
        fr: "sur cette machine",
        de: "auf diesem Rechner",
        it: "su questa macchina",
        ja: "このマシン上",
        zh: "在这台设备上",
        ko: "이 컴퓨터에서",
        ru: "на этом компьютере",
    }
    /// Status line: name used when the host is unknown.
    analysed_configured_provider {
        en: "configured provider",
        pt: "provedor configurado",
        es: "proveedor configurado",
        fr: "fournisseur configuré",
        de: "konfigurierter Anbieter",
        it: "provider configurato",
        ja: "設定済みのプロバイダー",
        zh: "已配置的提供商",
        ko: "설정한 공급자",
        ru: "настроенный провайдер",
    }
    /// Preview note: the address is local.
    destination_local_note {
        en: "Local address: the excerpts don’t leave this machine.",
        pt: "Endereço local: os trechos não saem desta máquina.",
        es: "Dirección local: los fragmentos no salen de esta máquina.",
        fr: "Adresse locale : les extraits ne quittent pas cette machine.",
        de: "Lokale Adresse: Die Ausschnitte verlassen diesen Rechner nicht.",
        it: "Indirizzo locale: gli estratti non escono da questa macchina.",
        ja: "ローカルのアドレス：抜粋はこのマシンの外に出ません。",
        zh: "本地地址：片段不会离开这台设备。",
        ko: "로컬 주소예요. 발췌한 내용은 이 컴퓨터 밖으로 나가지 않아요.",
        ru: "Локальный адрес: фрагменты не покидают этот компьютер.",
    }
    /// Preview note: each analysis counts toward the ChatGPT plan.
    destination_chatgpt_note {
        en: "Each analysis counts toward your ChatGPT plan’s usage.",
        pt: "Cada análise conta no uso do seu plano do ChatGPT.",
        es: "Cada análisis cuenta en el uso de tu plan de ChatGPT.",
        fr: "Chaque analyse compte dans l’utilisation de votre forfait ChatGPT.",
        de: "Jede Analyse zählt zur Nutzung deines ChatGPT-Tarifs.",
        it: "Ogni analisi conta nell’utilizzo del tuo piano ChatGPT.",
        ja: "分析のたびに、ChatGPTのプランの使用量に加算されます。",
        zh: "每次分析都会计入你的 ChatGPT 套餐用量。",
        ko: "분석할 때마다 ChatGPT 플랜 사용량에 포함돼요.",
        ru: "Каждый анализ учитывается в использовании вашего тарифа ChatGPT.",
    }
    /// Preview note: OpenCode forwards the excerpts.
    destination_opencode_note {
        en: "OpenCode forwards the excerpts to the provider of the model you chose.",
        pt: "O OpenCode repassa os trechos ao provedor do modelo escolhido.",
        es: "OpenCode reenvía los fragmentos al proveedor del modelo elegido.",
        fr: "OpenCode transmet les extraits au fournisseur du modèle choisi.",
        de: "OpenCode leitet die Ausschnitte an den Anbieter des gewählten Modells weiter.",
        it: "OpenCode inoltra gli estratti al provider del modello scelto.",
        ja: "OpenCodeは、選んだモデルのプロバイダーに抜粋を転送します。",
        zh: "OpenCode 会把片段转交给你所选模型的提供商。",
        ko: "OpenCode가 고른 모델의 공급자에게 발췌한 내용을 전달해요.",
        ru: "OpenCode передаёт фрагменты провайдеру выбранной модели.",
    }
    /// Consent checklist: consent step.
    step_consent_title {
        en: "Consent",
        pt: "Consentimento",
        es: "Consentimiento",
        fr: "Consentement",
        de: "Einwilligung",
        it: "Consenso",
        ja: "同意",
        zh: "同意",
        ko: "동의",
        ru: "Согласие",
    }
    /// Consent checklist: what consent does.
    step_consent_hint {
        en: "Turns on external calls",
        pt: "Liga as chamadas externas",
        es: "Activa las llamadas externas",
        fr: "Active les appels externes",
        de: "Schaltet externe Aufrufe ein",
        it: "Attiva le chiamate esterne",
        ja: "外部への呼び出しを有効にします",
        zh: "启用外部调用",
        ko: "외부 호출을 켜요",
        ru: "Включает внешние вызовы",
    }
    /// Consent checklist: no key needed.
    step_no_key_title {
        en: "No key",
        pt: "Sem chave",
        es: "Sin clave",
        fr: "Sans clé",
        de: "Kein Schlüssel",
        it: "Nessuna chiave",
        ja: "キーなし",
        zh: "无需密钥",
        ko: "키 없음",
        ru: "Без ключа",
    }
    /// Consent checklist: no key needed because the address is local.
    step_no_key_hint {
        en: "Local address",
        pt: "Endereço local",
        es: "Dirección local",
        fr: "Adresse locale",
        de: "Lokale Adresse",
        it: "Indirizzo locale",
        ja: "ローカルのアドレス",
        zh: "本地地址",
        ko: "로컬 주소",
        ru: "Локальный адрес",
    }
    /// Consent checklist: key in the system credential store.
    step_key_title {
        en: "Key in the credential store",
        pt: "Chave no cofre",
        es: "Clave en el almacén",
        fr: "Clé dans le coffre",
        de: "Schlüssel im Speicher",
        it: "Chiave nell’archivio",
        ja: "キーを保管済み",
        zh: "密钥已存入凭据存储",
        ko: "저장소에 키 있음",
        ru: "Ключ в хранилище",
    }
    /// Consent checklist: where the key is kept.
    step_key_hint {
        en: "Stored in the system",
        pt: "Guardada no sistema",
        es: "Guardada en el sistema",
        fr: "Conservée dans le système",
        de: "Im System gespeichert",
        it: "Salvata nel sistema",
        ja: "システムに保存",
        zh: "保存在系统中",
        ko: "시스템에 저장됨",
        ru: "Хранится в системе",
    }
    /// Consent checklist: configuration saved.
    step_saved_title {
        en: "Configuration saved",
        pt: "Configuração salva",
        es: "Configuración guardada",
        fr: "Configuration enregistrée",
        de: "Konfiguration gespeichert",
        it: "Configurazione salvata",
        ja: "設定を保存済み",
        zh: "配置已保存",
        ko: "설정 저장됨",
        ru: "Настройка сохранена",
    }
    /// Consent checklist: what the saved configuration covers (address provider).
    step_saved_hint_address {
        en: "Address, model and limit",
        pt: "Endereço, modelo e limite",
        es: "Dirección, modelo y límite",
        fr: "Adresse, modèle et limite",
        de: "Adresse, Modell und Limit",
        it: "Indirizzo, modello e limite",
        ja: "アドレス、モデル、上限",
        zh: "地址、模型和上限",
        ko: "주소, 모델, 한도",
        ru: "Адрес, модель и лимит",
    }
    /// Consent checklist: what the saved configuration covers (OpenCode).
    step_saved_hint_plan {
        en: "Plan and model",
        pt: "Plano e modelo",
        es: "Plan y modelo",
        fr: "Forfait et modèle",
        de: "Tarif und Modell",
        it: "Piano e modello",
        ja: "プランとモデル",
        zh: "套餐和模型",
        ko: "플랜과 모델",
        ru: "Тариф и модель",
    }
    /// Consent checklist: ChatGPT model saved.
    step_model_saved_title {
        en: "Model saved",
        pt: "Modelo salvo",
        es: "Modelo guardado",
        fr: "Modèle enregistré",
        de: "Modell gespeichert",
        it: "Modello salvato",
        ja: "モデルを保存済み",
        zh: "模型已保存",
        ko: "모델 저장됨",
        ru: "Модель сохранена",
    }
    /// Consent checklist: what the saved ChatGPT model covers.
    step_model_saved_hint {
        en: "Model and limit",
        pt: "Modelo e limite",
        es: "Modelo y límite",
        fr: "Modèle et limite",
        de: "Modell und Limit",
        it: "Modello e limite",
        ja: "モデルと上限",
        zh: "模型和上限",
        ko: "모델과 한도",
        ru: "Модель и лимит",
    }
    /// Consent checklist: ChatGPT account connected.
    step_account_title {
        en: "Account connected",
        pt: "Conta conectada",
        es: "Cuenta conectada",
        fr: "Compte connecté",
        de: "Konto verbunden",
        it: "Account collegato",
        ja: "アカウント接続済み",
        zh: "账户已连接",
        ko: "계정 연결됨",
        ru: "Учётная запись подключена",
    }
    /// Consent checklist: plan usage is allowed.
    step_account_hint {
        en: "Plan usage allowed",
        pt: "Uso do plano permitido",
        es: "Uso del plan permitido",
        fr: "Utilisation du forfait autorisée",
        de: "Tarifnutzung erlaubt",
        it: "Utilizzo del piano consentito",
        ja: "プランの使用を許可済み",
        zh: "已允许使用套餐",
        ko: "플랜 사용 허용됨",
        ru: "Использование тарифа разрешено",
    }
    /// ChatGPT account toast: the sign-in did not finish.
    account_sign_in_cancelled {
        en: "Sign-in not completed.",
        pt: "Login não concluído.",
        es: "Inicio de sesión no completado.",
        fr: "Connexion non terminée.",
        de: "Anmeldung nicht abgeschlossen.",
        it: "Accesso non completato.",
        ja: "サインインが完了しませんでした。",
        zh: "登录未完成。",
        ko: "로그인을 마치지 못했어요.",
        ru: "Вход не завершён.",
    }
    /// ChatGPT account error: the sign-in ended during another operation.
    account_err_busy {
        en: "The sign-in finished during another operation. Sign in again.",
        pt: "O login terminou durante outra operação. Entre de novo.",
        es: "El inicio de sesión terminó durante otra operación. Inicia sesión de nuevo.",
        fr: "La connexion s’est terminée pendant une autre opération. Reconnectez-vous.",
        de: "Die Anmeldung wurde während eines anderen Vorgangs abgeschlossen. Melde dich \
            erneut an.",
        it: "L’accesso si è concluso durante un’altra operazione. Accedi di nuovo.",
        ja: "別の操作の最中にサインインが終わりました。もう一度サインインしてください。",
        zh: "登录在另一项操作进行期间完成了。请重新登录。",
        ko: "다른 작업이 진행되는 동안 로그인이 끝났어요. 다시 로그인해 주세요.",
        ru: "Вход завершился во время другой операции. Войдите снова.",
    }
    /// ChatGPT account toast: signed in.
    account_notice_connected {
        en: "ChatGPT account connected.",
        pt: "Conta ChatGPT conectada.",
        es: "Cuenta de ChatGPT conectada.",
        fr: "Compte ChatGPT connecté.",
        de: "ChatGPT-Konto verbunden.",
        it: "Account ChatGPT collegato.",
        ja: "ChatGPTアカウントを接続しました。",
        zh: "ChatGPT 账户已连接。",
        ko: "ChatGPT 계정을 연결했어요.",
        ru: "Учётная запись ChatGPT подключена.",
    }
    /// ChatGPT account toast: signed out but OpenAI did not confirm the revocation.
    account_notice_signed_out_unconfirmed {
        en: "Account disconnected here. OpenAI didn’t confirm the revocation; also remove \
            access at chatgpt.com.",
        pt: "Conta desconectada aqui. A OpenAI não confirmou a revogação; remova o acesso \
            também em chatgpt.com.",
        es: "Cuenta desconectada aquí. OpenAI no confirmó la revocación; quita también el \
            acceso en chatgpt.com.",
        fr: "Compte déconnecté ici. OpenAI n’a pas confirmé la révocation ; supprimez aussi \
            l’accès sur chatgpt.com.",
        de: "Konto hier getrennt. OpenAI hat den Widerruf nicht bestätigt; entziehe den Zugriff \
            auch auf chatgpt.com.",
        it: "Account scollegato qui. OpenAI non ha confermato la revoca; rimuovi l’accesso \
            anche su chatgpt.com.",
        ja: "ここではアカウントを切断しました。OpenAIは取り消しを確認できませんでした。chatgpt.comでもアクセスを削除してください。",
        zh: "已在此处断开账户。OpenAI 未确认撤销；请同时在 chatgpt.com 中移除访问权限。",
        ko: "여기서는 계정 연결을 끊었어요. OpenAI가 철회를 확인하지 않았으니 chatgpt.com에서도 접근 권한을 제거하세요.",
        ru: "Учётная запись отключена здесь. OpenAI не подтвердил отзыв доступа; отзовите его \
            также на chatgpt.com.",
    }
    /// ChatGPT account toast: signed out and token deleted.
    account_notice_signed_out {
        en: "ChatGPT account disconnected and token deleted from the credential store.",
        pt: "Conta ChatGPT desconectada e token apagado do cofre.",
        es: "Cuenta de ChatGPT desconectada y token borrado del almacén de credenciales.",
        fr: "Compte ChatGPT déconnecté et jeton supprimé du coffre d’identifiants.",
        de: "ChatGPT-Konto getrennt und Token aus dem Anmeldeinformationsspeicher gelöscht.",
        it: "Account ChatGPT scollegato e token eliminato dall’archivio credenziali.",
        ja: "ChatGPTアカウントを切断し、トークンを資格情報ストアから削除しました。",
        zh: "已断开 ChatGPT 账户，并从凭据存储中删除令牌。",
        ko: "ChatGPT 계정 연결을 끊고 자격 증명 저장소에서 토큰을 삭제했어요.",
        ru: "Учётная запись ChatGPT отключена, токен удалён из хранилища учётных данных.",
    }
    /// Provider form: label of the local model presets.
    models_on_this_machine {
        en: "Model on this machine:",
        pt: "Modelo nesta máquina:",
        es: "Modelo en esta máquina:",
        fr: "Modèle sur cette machine :",
        de: "Modell auf diesem Rechner:",
        it: "Modello su questa macchina:",
        ja: "このマシン上のモデル：",
        zh: "本机上的模型：",
        ko: "이 컴퓨터의 모델:",
        ru: "Модель на этом компьютере:",
    }
    /// Provider form: list models button while loading.
    models_searching {
        en: "Searching…",
        pt: "Buscando…",
        es: "Buscando…",
        fr: "Recherche…",
        de: "Suche läuft…",
        it: "Ricerca…",
        ja: "検索中…",
        zh: "正在查找…",
        ko: "찾는 중…",
        ru: "Поиск…",
    }
    /// Provider form: list models button.
    models_list {
        en: "List models",
        pt: "Listar modelos",
        es: "Listar modelos",
        fr: "Lister les modèles",
        de: "Modelle auflisten",
        it: "Elenca modelli",
        ja: "モデルを一覧表示",
        zh: "列出模型",
        ko: "모델 목록 보기",
        ru: "Показать модели",
    }
    /// Provider form: label of the OpenCode plan choice.
    models_plan_label {
        en: "Plan",
        pt: "Plano",
        es: "Plan",
        fr: "Forfait",
        de: "Tarif",
        it: "Piano",
        ja: "プラン",
        zh: "套餐",
        ko: "플랜",
        ru: "Тариф",
    }
    /// Provider form: screen reader label of the OpenCode plan choice.
    models_plan_group_label {
        en: "OpenCode plan",
        pt: "Plano do OpenCode",
        es: "Plan de OpenCode",
        fr: "Forfait OpenCode",
        de: "OpenCode-Tarif",
        it: "Piano OpenCode",
        ja: "OpenCodeのプラン",
        zh: "OpenCode 套餐",
        ko: "OpenCode 플랜",
        ru: "Тариф OpenCode",
    }
    /// Provider form: the destination offers no models.
    models_none {
        en: "No models available at that destination. Type the model name.",
        pt: "Nenhum modelo disponível nesse destino. Digite o nome do modelo.",
        es: "No hay modelos disponibles en ese destino. Escribe el nombre del modelo.",
        fr: "Aucun modèle disponible à cette destination. Saisissez le nom du modèle.",
        de: "An diesem Ziel sind keine Modelle verfügbar. Gib den Modellnamen ein.",
        it: "Nessun modello disponibile in questa destinazione. Digita il nome del modello.",
        ja: "この送信先で使えるモデルはありません。モデル名を入力してください。",
        zh: "该目的地没有可用的模型。请输入模型名称。",
        ko: "이 대상에서 쓸 수 있는 모델이 없어요. 모델 이름을 입력하세요.",
        ru: "В этом месте нет доступных моделей. Введите название модели.",
    }
    /// Provider form: screen reader label of the model list.
    models_group_label {
        en: "Available models",
        pt: "Modelos disponíveis",
        es: "Modelos disponibles",
        fr: "Modèles disponibles",
        de: "Verfügbare Modelle",
        it: "Modelli disponibili",
        ja: "利用できるモデル",
        zh: "可用的模型",
        ko: "사용할 수 있는 모델",
        ru: "Доступные модели",
    }
    /// Key card: button when a key is already stored.
    key_replace {
        en: "Replace key",
        pt: "Substituir chave",
        es: "Reemplazar clave",
        fr: "Remplacer la clé",
        de: "Schlüssel ersetzen",
        it: "Sostituisci chiave",
        ja: "キーを置き換える",
        zh: "替换密钥",
        ko: "키 교체",
        ru: "Заменить ключ",
    }
    /// Key card: button that stores the key.
    key_store {
        en: "Store in the credential store",
        pt: "Guardar no cofre",
        es: "Guardar en el almacén",
        fr: "Enregistrer dans le coffre",
        de: "Im Speicher ablegen",
        it: "Salva nell’archivio",
        ja: "資格情報ストアに保存",
        zh: "存入凭据存储",
        ko: "저장소에 보관",
        ru: "Сохранить в хранилище",
    }
    /// Key card: status, the key is stored.
    key_state_stored {
        en: "Stored in the credential store",
        pt: "Guardada no cofre",
        es: "Guardada en el almacén",
        fr: "Enregistrée dans le coffre",
        de: "Im Speicher abgelegt",
        it: "Salvata nell’archivio",
        ja: "資格情報ストアに保存済み",
        zh: "已存入凭据存储",
        ko: "저장소에 보관됨",
        ru: "Хранится в хранилище",
    }
    /// Key card: status, a key is optional at this address.
    key_state_optional {
        en: "Optional at this address",
        pt: "Opcional neste endereço",
        es: "Opcional en esta dirección",
        fr: "Facultative à cette adresse",
        de: "An dieser Adresse optional",
        it: "Facoltativa a questo indirizzo",
        ja: "このアドレスでは任意",
        zh: "此地址下为可选",
        ko: "이 주소에서는 선택 사항",
        ru: "Для этого адреса необязателен",
    }
    /// Key card: status, no key.
    key_state_none {
        en: "No key",
        pt: "Nenhuma chave",
        es: "Ninguna clave",
        fr: "Aucune clé",
        de: "Kein Schlüssel",
        it: "Nessuna chiave",
        ja: "キーなし",
        zh: "无密钥",
        ko: "키 없음",
        ru: "Ключа нет",
    }
    /// Key card title for OpenCode.
    key_opencode_title {
        en: "OpenCode key",
        pt: "Chave do OpenCode",
        es: "Clave de OpenCode",
        fr: "Clé OpenCode",
        de: "OpenCode-Schlüssel",
        it: "Chiave di OpenCode",
        ja: "OpenCodeのキー",
        zh: "OpenCode 密钥",
        ko: "OpenCode 키",
        ru: "Ключ OpenCode",
    }
    /// Key card title for the provider.
    key_provider_title {
        en: "Provider key",
        pt: "Chave do provedor",
        es: "Clave del proveedor",
        fr: "Clé du fournisseur",
        de: "Schlüssel des Anbieters",
        it: "Chiave del provider",
        ja: "プロバイダーのキー",
        zh: "提供商密钥",
        ko: "공급자 키",
        ru: "Ключ провайдера",
    }
    /// Key card: link that opens the OpenCode console.
    key_create {
        en: "Create a key",
        pt: "Criar uma chave",
        es: "Crear una clave",
        fr: "Créer une clé",
        de: "Schlüssel erstellen",
        it: "Crea una chiave",
        ja: "キーを作成",
        zh: "创建密钥",
        ko: "키 만들기",
        ru: "Создать ключ",
    }
    /// ChatGPT account card: closes the plan usage notice.
    account_got_it {
        en: "Got it",
        pt: "Entendi",
        es: "Entendido",
        fr: "Compris",
        de: "Verstanden",
        it: "Ho capito",
        ja: "了解",
        zh: "知道了",
        ko: "알겠어요",
        ru: "Понятно",
    }
    /// ChatGPT account card: title of the plan usage notice.
    account_plan_notice_title {
        en: "You’re using your ChatGPT plan",
        pt: "Você está usando o seu plano do ChatGPT",
        es: "Estás usando tu plan de ChatGPT",
        fr: "Vous utilisez votre forfait ChatGPT",
        de: "Du nutzt deinen ChatGPT-Tarif",
        it: "Stai usando il tuo piano ChatGPT",
        ja: "ChatGPTのプランを使用中です",
        zh: "你正在使用你的 ChatGPT 套餐",
        ko: "ChatGPT 플랜을 사용 중이에요",
        ru: "Вы используете свой тариф ChatGPT",
    }
    /// ChatGPT account card: reopens the sign-in page.
    account_reopen_browser {
        en: "Open the browser again",
        pt: "Abrir o navegador de novo",
        es: "Abrir el navegador de nuevo",
        fr: "Rouvrir le navigateur",
        de: "Browser erneut öffnen",
        it: "Riapri il browser",
        ja: "ブラウザーをもう一度開く",
        zh: "重新打开浏览器",
        ko: "브라우저 다시 열기",
        ru: "Открыть браузер снова",
    }
    /// ChatGPT account card: plan usage allowed.
    account_pill_plan_on {
        en: "Using the ChatGPT plan",
        pt: "Usando o plano do ChatGPT",
        es: "Usando el plan de ChatGPT",
        fr: "Utilisation du forfait ChatGPT",
        de: "ChatGPT-Tarif in Nutzung",
        it: "Piano ChatGPT in uso",
        ja: "ChatGPTのプランを使用中",
        zh: "正在使用 ChatGPT 套餐",
        ko: "ChatGPT 플랜 사용 중",
        ru: "Используется тариф ChatGPT",
    }
    /// ChatGPT account card: plan usage not allowed.
    account_pill_plan_off {
        en: "Plan usage not allowed",
        pt: "Uso do plano não permitido",
        es: "Uso del plan no permitido",
        fr: "Utilisation du forfait non autorisée",
        de: "Tarifnutzung nicht erlaubt",
        it: "Utilizzo del piano non consentito",
        ja: "プランの使用は未許可",
        zh: "未允许使用套餐",
        ko: "플랜 사용이 허용되지 않음",
        ru: "Использование тарифа не разрешено",
    }
    /// ChatGPT account card: name shown when the account has no email.
    account_connected {
        en: "Account connected",
        pt: "Conta conectada",
        es: "Cuenta conectada",
        fr: "Compte connecté",
        de: "Konto verbunden",
        it: "Account collegato",
        ja: "アカウント接続済み",
        zh: "账户已连接",
        ko: "계정 연결됨",
        ru: "Учётная запись подключена",
    }
    /// ChatGPT account card: plan usage is off.
    account_sign_in_again_hint {
        en: "Sign in again and allow plan usage to extract with it.",
        pt: "Entre de novo e permita o uso do plano para extrair com ele.",
        es: "Inicia sesión de nuevo y permite el uso del plan para extraer con él.",
        fr: "Reconnectez-vous et autorisez l’utilisation du forfait pour extraire avec lui.",
        de: "Melde dich erneut an und erlaube die Tarifnutzung, um damit zu extrahieren.",
        it: "Accedi di nuovo e consenti l’uso del piano per estrarre con esso.",
        ja: "もう一度サインインし、プランの使用を許可すると、プランで抽出できます。",
        zh: "请重新登录并允许使用套餐，才能用它进行提取。",
        ko: "다시 로그인해서 플랜 사용을 허용하면 플랜으로 추출할 수 있어요.",
        ru: "Войдите снова и разрешите использование тарифа, чтобы извлекать данные по нему.",
    }
    /// ChatGPT account card: sign out.
    account_sign_out {
        en: "Sign out",
        pt: "Sair da conta",
        es: "Cerrar sesión",
        fr: "Se déconnecter",
        de: "Abmelden",
        it: "Esci dall’account",
        ja: "サインアウト",
        zh: "退出账户",
        ko: "로그아웃",
        ru: "Выйти из учётной записи",
    }
    /// ChatGPT account card: sign in again.
    account_sign_in_again {
        en: "Sign in again",
        pt: "Entrar de novo",
        es: "Iniciar sesión de nuevo",
        fr: "Se reconnecter",
        de: "Erneut anmelden",
        it: "Accedi di nuovo",
        ja: "もう一度サインイン",
        zh: "重新登录",
        ko: "다시 로그인",
        ru: "Войти снова",
    }
    /// ChatGPT account card: sign-in button while the browser opens.
    account_opening_browser {
        en: "Opening the browser…",
        pt: "Abrindo o navegador…",
        es: "Abriendo el navegador…",
        fr: "Ouverture du navigateur…",
        de: "Browser wird geöffnet…",
        it: "Apertura del browser…",
        ja: "ブラウザーを開いています…",
        zh: "正在打开浏览器…",
        ko: "브라우저 여는 중…",
        ru: "Открываем браузер…",
    }
    /// ChatGPT account card: sign-in button.
    account_continue {
        en: "Continue with ChatGPT",
        pt: "Continuar com o ChatGPT",
        es: "Continuar con ChatGPT",
        fr: "Continuer avec ChatGPT",
        de: "Weiter mit ChatGPT",
        it: "Continua con ChatGPT",
        ja: "ChatGPTで続ける",
        zh: "使用 ChatGPT 继续",
        ko: "ChatGPT로 계속",
        ru: "Продолжить с ChatGPT",
    }
    /// ChatGPT account card: sign-in not available in this build.
    account_unavailable {
        en: "Signing in with ChatGPT isn’t available here.",
        pt: "O login com o ChatGPT não está disponível aqui.",
        es: "El inicio de sesión con ChatGPT no está disponible aquí.",
        fr: "La connexion avec ChatGPT n’est pas disponible ici.",
        de: "Die Anmeldung mit ChatGPT ist hier nicht verfügbar.",
        it: "L’accesso con ChatGPT non è disponibile qui.",
        ja: "ここではChatGPTでのサインインは使えません。",
        zh: "此处无法使用 ChatGPT 登录。",
        ko: "여기서는 ChatGPT로 로그인할 수 없어요.",
        ru: "Вход через ChatGPT здесь недоступен.",
    }
    /// ChatGPT account card title.
    account_title {
        en: "ChatGPT account",
        pt: "Conta ChatGPT",
        es: "Cuenta de ChatGPT",
        fr: "Compte ChatGPT",
        de: "ChatGPT-Konto",
        it: "Account ChatGPT",
        ja: "ChatGPTアカウント",
        zh: "ChatGPT 账户",
        ko: "ChatGPT 계정",
        ru: "Учётная запись ChatGPT",
    }
    /// ChatGPT account card subtitle.
    account_body {
        en: "Signing in happens in the browser; your password never passes through xemnas.",
        pt: "O login acontece no navegador; a senha nunca passa pela xemnas.",
        es: "El inicio de sesión ocurre en el navegador; tu contraseña nunca pasa por xemnas.",
        fr: "La connexion se fait dans le navigateur ; votre mot de passe ne passe jamais par \
            xemnas.",
        de: "Die Anmeldung erfolgt im Browser; dein Passwort läuft nie über xemnas.",
        it: "L’accesso avviene nel browser; la password non passa mai da xemnas.",
        ja: "サインインはブラウザーで行われ、パスワードがxemnasを通ることはありません。",
        zh: "登录在浏览器中进行；你的密码从不经过 xemnas。",
        ko: "로그인은 브라우저에서 이뤄지며, 비밀번호는 xemnas를 거치지 않아요.",
        ru: "Вход происходит в браузере; ваш пароль никогда не проходит через xemnas.",
    }
    /// OpenCode error: the integration status could not be read.
    opencode_err_status {
        en: "Couldn’t read the integration status.",
        pt: "Não foi possível ler o estado da integração.",
        es: "No se pudo leer el estado de la integración.",
        fr: "Impossible de lire l’état de l’intégration.",
        de: "Der Status der Integration konnte nicht gelesen werden.",
        it: "Impossibile leggere lo stato dell’integrazione.",
        ja: "連携の状態を読み取れませんでした。",
        zh: "无法读取集成状态。",
        ko: "연동 상태를 읽을 수 없어요.",
        ru: "Не удалось прочитать состояние интеграции.",
    }
    /// OpenCode error: the connection test could not run.
    opencode_err_check {
        en: "Couldn’t run the connection test.",
        pt: "Não foi possível executar o teste de conexão.",
        es: "No se pudo ejecutar la prueba de conexión.",
        fr: "Impossible d’exécuter le test de connexion.",
        de: "Der Verbindungstest konnte nicht ausgeführt werden.",
        it: "Impossibile eseguire il test di connessione.",
        ja: "接続テストを実行できませんでした。",
        zh: "无法运行连接测试。",
        ko: "연결 테스트를 실행할 수 없어요.",
        ru: "Не удалось выполнить проверку подключения.",
    }
    /// OpenCode error: stalled captures could not be moved.
    opencode_err_retry {
        en: "Couldn’t move the stalled captures.",
        pt: "Não foi possível mover as capturas paradas.",
        es: "No se pudieron mover las capturas detenidas.",
        fr: "Impossible de déplacer les captures bloquées.",
        de: "Die angehaltenen Erfassungen konnten nicht verschoben werden.",
        it: "Impossibile spostare le acquisizioni ferme.",
        ja: "停止中のキャプチャを移動できませんでした。",
        zh: "无法移动停滞的捕获。",
        ko: "멈춘 캡처를 옮길 수 없어요.",
        ru: "Не удалось переместить застрявшие захваты.",
    }
    /// OpenCode status: captures are arriving.
    opencode_receiving_title {
        en: "Receiving captures",
        pt: "Recebendo capturas",
        es: "Recibiendo capturas",
        fr: "Réception de captures",
        de: "Erfassungen werden empfangen",
        it: "Ricezione di acquisizioni",
        ja: "キャプチャを受信中",
        zh: "正在接收捕获",
        ko: "캡처 수신 중",
        ru: "Захваты поступают",
    }
    /// OpenCode status: no capture yet.
    opencode_awaiting_title {
        en: "Waiting for the first capture",
        pt: "Aguardando a primeira captura",
        es: "Esperando la primera captura",
        fr: "En attente de la première capture",
        de: "Warten auf die erste Erfassung",
        it: "In attesa della prima acquisizione",
        ja: "最初のキャプチャを待っています",
        zh: "正在等待第一个捕获",
        ko: "첫 캡처를 기다리는 중",
        ru: "Ожидание первого захвата",
    }
    /// OpenCode status: what to do to get the first capture.
    opencode_awaiting_body {
        en: "The local API is active. Finish a turn in OpenCode, with the xemnas plugin, inside \
            a registered project.",
        pt: "A API local está ativa. Conclua um turno no OpenCode, com o plugin do xemnas, \
            dentro de um projeto cadastrado.",
        es: "La API local está activa. Termina un turno en OpenCode, con el plugin de xemnas, \
            dentro de un proyecto registrado.",
        fr: "L’API locale est active. Terminez un tour dans OpenCode, avec le plugin xemnas, \
            dans un projet enregistré.",
        de: "Die lokale API ist aktiv. Schließe in OpenCode mit dem xemnas-Plugin einen \
            Durchgang innerhalb eines registrierten Projekts ab.",
        it: "L’API locale è attiva. Completa un turno in OpenCode, con il plugin di xemnas, \
            dentro un progetto registrato.",
        ja: "ローカルAPIは有効です。登録済みのプロジェクト内で、xemnasのプラグインを入れたOpenCodeで1ターンを完了してください。",
        zh: "本地 API 已启用。请在已登记的项目中，用装有 xemnas 插件的 OpenCode 完成一轮对话。",
        ko: "로컬 API가 켜져 있어요. 등록된 프로젝트 안에서 xemnas 플러그인을 쓰는 OpenCode로 한 턴을 마치세요.",
        ru: "Локальный API активен. Завершите ход в OpenCode с плагином xemnas внутри \
            зарегистрированного проекта.",
    }
    /// OpenCode status: the local API is down.
    opencode_api_down_title {
        en: "Local API inactive",
        pt: "API local inativa",
        es: "API local inactiva",
        fr: "API locale inactive",
        de: "Lokale API inaktiv",
        it: "API locale inattiva",
        ja: "ローカルAPIが無効",
        zh: "本地 API 未启用",
        ko: "로컬 API 꺼짐",
        ru: "Локальный API неактивен",
    }
    /// OpenCode status: what happens while the local API is down.
    opencode_api_down_body {
        en: "Captures wait in the outbox and are imported the next time the app opens. Restart \
            xemnas to turn the API back on.",
        pt: "As capturas esperam na outbox e são importadas na próxima abertura do app. \
            Reinicie o xemnas para reativar a API.",
        es: "Las capturas esperan en la outbox y se importan la próxima vez que se abra la app. \
            Reinicia xemnas para reactivar la API.",
        fr: "Les captures attendent dans l’outbox et sont importées à la prochaine ouverture de \
            l’app. Redémarrez xemnas pour réactiver l’API.",
        de: "Erfassungen warten in der Outbox und werden beim nächsten Öffnen der App \
            importiert. Starte xemnas neu, um die API wieder zu aktivieren.",
        it: "Le acquisizioni restano nella outbox e vengono importate alla prossima apertura \
            dell’app. Riavvia xemnas per riattivare l’API.",
        ja: "キャプチャはアウトボックスで待機し、次回アプリを開いたときに取り込まれます。APIを再び有効にするには、xemnasを再起動してください。",
        zh: "捕获会在发件箱中等待，并在下次打开应用时导入。重启 xemnas 即可重新启用 API。",
        ko: "캡처는 아웃박스에서 기다렸다가 다음에 앱을 열 때 가져와요. API를 다시 켜려면 xemnas를 다시 시작하세요.",
        ru: "Захваты ждут в outbox и импортируются при следующем открытии приложения. \
            Перезапустите xemnas, чтобы снова включить API.",
    }
    /// OpenCode connection test: what the test checks.
    opencode_test_body {
        en: "Checks the local API, the discovery file, the session token, the outbox and the \
            captures received.",
        pt: "Verifica a API local, o arquivo de descoberta, o token de sessão, a outbox e as \
            capturas recebidas.",
        es: "Comprueba la API local, el archivo de descubrimiento, el token de sesión, la \
            outbox y las capturas recibidas.",
        fr: "Vérifie l’API locale, le fichier de découverte, le jeton de session, l’outbox et \
            les captures reçues.",
        de: "Prüft die lokale API, die Discovery-Datei, das Sitzungstoken, die Outbox und die \
            empfangenen Erfassungen.",
        it: "Verifica l’API locale, il file di discovery, il token di sessione, la outbox e le \
            acquisizioni ricevute.",
        ja: "ローカルAPI、ディスカバリーファイル、セッショントークン、アウトボックス、受信したキャプチャを確認します。",
        zh: "检查本地 API、发现文件、会话令牌、发件箱和已收到的捕获。",
        ko: "로컬 API, 디스커버리 파일, 세션 토큰, 아웃박스, 받은 캡처를 확인해요.",
        ru: "Проверяет локальный API, файл обнаружения, токен сессии, outbox и полученные захваты.",
    }
    /// OpenCode connection test: button and screen reader label.
    opencode_test {
        en: "Test connection",
        pt: "Testar conexão",
        es: "Probar conexión",
        fr: "Tester la connexion",
        de: "Verbindung testen",
        it: "Testa la connessione",
        ja: "接続をテスト",
        zh: "测试连接",
        ko: "연결 테스트",
        ru: "Проверить подключение",
    }
    /// OpenCode connection test: button while running.
    opencode_testing {
        en: "Testing…",
        pt: "Testando…",
        es: "Probando…",
        fr: "Test en cours…",
        de: "Test läuft…",
        it: "Test in corso…",
        ja: "テスト中…",
        zh: "正在测试…",
        ko: "테스트 중…",
        ru: "Проверка…",
    }
    /// OpenCode connection test: button after a first run.
    opencode_test_again {
        en: "Test again",
        pt: "Testar de novo",
        es: "Probar de nuevo",
        fr: "Tester à nouveau",
        de: "Erneut testen",
        it: "Testa di nuovo",
        ja: "もう一度テスト",
        zh: "重新测试",
        ko: "다시 테스트",
        ru: "Проверить снова",
    }
    /// OpenCode connection test card title.
    opencode_test_title {
        en: "Connection test",
        pt: "Teste de conexão",
        es: "Prueba de conexión",
        fr: "Test de connexion",
        de: "Verbindungstest",
        it: "Test di connessione",
        ja: "接続テスト",
        zh: "连接测试",
        ko: "연결 테스트",
        ru: "Проверка подключения",
    }
    /// OpenCode connection test card subtitle.
    opencode_test_card_body {
        en: "The path a capture takes from OpenCode to this app.",
        pt: "O caminho que uma captura percorre do OpenCode até este app.",
        es: "El camino que recorre una captura desde OpenCode hasta esta app.",
        fr: "Le chemin qu’une capture suit d’OpenCode jusqu’à cette app.",
        de: "Der Weg, den eine Erfassung von OpenCode bis zu dieser App nimmt.",
        it: "Il percorso che un’acquisizione compie da OpenCode a questa app.",
        ja: "キャプチャがOpenCodeからこのアプリまで通る経路。",
        zh: "捕获从 OpenCode 到此应用所经过的路径。",
        ko: "캡처가 OpenCode에서 이 앱까지 오는 경로예요.",
        ru: "Путь, который проходит захват от OpenCode до этого приложения.",
    }
    /// OpenCode adapters card: none delivered captures yet.
    opencode_no_adapters {
        en: "No adapter has sent captures yet.",
        pt: "Nenhum adapter enviou capturas ainda.",
        es: "Ningún adaptador ha enviado capturas todavía.",
        fr: "Aucun adaptateur n’a encore envoyé de captures.",
        de: "Noch kein Adapter hat Erfassungen gesendet.",
        it: "Nessun adapter ha ancora inviato acquisizioni.",
        ja: "まだキャプチャを送ったアダプターはありません。",
        zh: "还没有适配器发送过捕获。",
        ko: "아직 캡처를 보낸 어댑터가 없어요.",
        ru: "Ни один адаптер ещё не присылал захваты.",
    }
    /// OpenCode adapters card: the adapter version is compatible.
    opencode_compatible {
        en: "Compatible",
        pt: "Compatível",
        es: "Compatible",
        fr: "Compatible",
        de: "Kompatibel",
        it: "Compatibile",
        ja: "互換性あり",
        zh: "兼容",
        ko: "호환됨",
        ru: "Совместим",
    }
    /// OpenCode adapters card: the adapter did not report its version.
    opencode_version_unknown {
        en: "Version not reported",
        pt: "Versão não informada",
        es: "Versión no indicada",
        fr: "Version non indiquée",
        de: "Version nicht angegeben",
        it: "Versione non indicata",
        ja: "バージョン不明",
        zh: "未提供版本",
        ko: "버전 정보 없음",
        ru: "Версия не указана",
    }
    /// OpenCode adapters card title.
    opencode_adapters_title {
        en: "Adapters",
        pt: "Adapters",
        es: "Adaptadores",
        fr: "Adaptateurs",
        de: "Adapter",
        it: "Adapter",
        ja: "アダプター",
        zh: "适配器",
        ko: "어댑터",
        ru: "Адаптеры",
    }
    /// OpenCode adapters card subtitle.
    opencode_adapters_body {
        en: "Integrations that have already delivered captures to this app.",
        pt: "Integrações que já entregaram capturas a este app.",
        es: "Integraciones que ya han entregado capturas a esta app.",
        fr: "Intégrations qui ont déjà livré des captures à cette app.",
        de: "Integrationen, die dieser App bereits Erfassungen geliefert haben.",
        it: "Integrazioni che hanno già consegnato acquisizioni a questa app.",
        ja: "すでにこのアプリにキャプチャを届けた連携。",
        zh: "已向此应用送达过捕获的集成。",
        ko: "이미 이 앱에 캡처를 전달한 연동이에요.",
        ru: "Интеграции, которые уже доставили захваты в это приложение.",
    }
    /// OpenCode outbox: tile label, pending.
    opencode_outbox_pending {
        en: "Pending",
        pt: "Pendentes",
        es: "Pendientes",
        fr: "En attente",
        de: "Ausstehend",
        it: "In sospeso",
        ja: "保留中",
        zh: "待处理",
        ko: "대기 중",
        ru: "Ожидают",
    }
    /// OpenCode outbox: tile label, accepted.
    opencode_outbox_accepted {
        en: "Accepted",
        pt: "Aceitas",
        es: "Aceptadas",
        fr: "Acceptées",
        de: "Angenommen",
        it: "Accettate",
        ja: "受理済み",
        zh: "已接受",
        ko: "수락됨",
        ru: "Приняты",
    }
    /// OpenCode outbox: tile label, rejected.
    opencode_outbox_rejected {
        en: "Rejected",
        pt: "Rejeitadas",
        es: "Rechazadas",
        fr: "Rejetées",
        de: "Abgelehnt",
        it: "Rifiutate",
        ja: "却下済み",
        zh: "已拒绝",
        ko: "거부됨",
        ru: "Отклонены",
    }
    /// OpenCode outbox: tile label, stalled.
    opencode_outbox_stalled {
        en: "Stalled",
        pt: "Paradas",
        es: "Detenidas",
        fr: "Bloquées",
        de: "Angehalten",
        it: "Ferme",
        ja: "停止中",
        zh: "停滞",
        ko: "멈춤",
        ru: "Застряли",
    }
    /// OpenCode outbox card title.
    opencode_outbox_title {
        en: "File queue",
        pt: "Fila de arquivos",
        es: "Cola de archivos",
        fr: "File de fichiers",
        de: "Dateiwarteschlange",
        it: "Coda di file",
        ja: "ファイルキュー",
        zh: "文件队列",
        ko: "파일 대기열",
        ru: "Файловая очередь",
    }
    /// OpenCode outbox card subtitle.
    opencode_outbox_body {
        en: "Where the adapter keeps captures while the app is closed.",
        pt: "Onde o adapter guarda capturas enquanto o app está fechado.",
        es: "Donde el adaptador guarda las capturas mientras la app está cerrada.",
        fr: "Là où l’adaptateur garde les captures pendant que l’app est fermée.",
        de: "Wo der Adapter Erfassungen aufbewahrt, solange die App geschlossen ist.",
        it: "Dove l’adapter conserva le acquisizioni mentre l’app è chiusa.",
        ja: "アプリが閉じている間、アダプターがキャプチャを保管する場所。",
        zh: "应用关闭期间，适配器存放捕获的位置。",
        ko: "앱이 닫혀 있는 동안 어댑터가 캡처를 보관하는 곳이에요.",
        ru: "Где адаптер хранит захваты, пока приложение закрыто.",
    }
    /// OpenCode outbox: what stalled captures are.
    opencode_stalled_hint {
        en: "Stalled captures belong to projects that aren’t registered. Register the project \
            and send them back to the queue.",
        pt: "Paradas são capturas de projetos não cadastrados. Cadastre o projeto e mande-as de \
            volta para a fila.",
        es: "Las detenidas son capturas de proyectos no registrados. Registra el proyecto y \
            devuélvelas a la cola.",
        fr: "Les captures bloquées viennent de projets non enregistrés. Enregistrez le projet \
            et remettez-les dans la file.",
        de: "Angehaltene Erfassungen gehören zu nicht registrierten Projekten. Registriere das \
            Projekt und schicke sie zurück in die Warteschlange.",
        it: "Le ferme sono acquisizioni di progetti non registrati. Registra il progetto e \
            rimettile in coda.",
        ja: "停止中のキャプチャは、未登録のプロジェクトのものです。プロジェクトを登録して、キューに戻してください。",
        zh: "停滞的捕获来自尚未登记的项目。登记该项目后，再把它们送回队列。",
        ko: "멈춘 캡처는 등록되지 않은 프로젝트의 것이에요. 프로젝트를 등록한 뒤 대기열로 되돌리세요.",
        ru: "Застрявшие — это захваты из незарегистрированных проектов. Зарегистрируйте проект \
            и верните их в очередь.",
    }
    /// OpenCode outbox: screen reader label of the retry button.
    opencode_retry_label {
        en: "Resend stalled captures",
        pt: "Reenviar capturas paradas",
        es: "Reenviar capturas detenidas",
        fr: "Renvoyer les captures bloquées",
        de: "Angehaltene Erfassungen erneut senden",
        it: "Reinvia le acquisizioni ferme",
        ja: "停止中のキャプチャを再送",
        zh: "重新发送停滞的捕获",
        ko: "멈춘 캡처 다시 보내기",
        ru: "Отправить застрявшие захваты снова",
    }
    /// OpenCode outbox: retry button.
    opencode_retry {
        en: "Resend stalled",
        pt: "Reenviar paradas",
        es: "Reenviar detenidas",
        fr: "Renvoyer les bloquées",
        de: "Angehaltene senden",
        it: "Reinvia le ferme",
        ja: "停止中を再送",
        zh: "重发停滞项",
        ko: "멈춘 항목 다시 보내기",
        ru: "Отправить застрявшие",
    }
    /// OpenCode local connection card title.
    opencode_connection_title {
        en: "Local connection",
        pt: "Conexão local",
        es: "Conexión local",
        fr: "Connexion locale",
        de: "Lokale Verbindung",
        it: "Connessione locale",
        ja: "ローカル接続",
        zh: "本地连接",
        ko: "로컬 연결",
        ru: "Локальное подключение",
    }
    /// OpenCode local connection card subtitle.
    opencode_connection_body {
        en: "How the adapter finds this app.",
        pt: "Como o adapter encontra este app.",
        es: "Cómo el adaptador encuentra esta app.",
        fr: "Comment l’adaptateur trouve cette app.",
        de: "Wie der Adapter diese App findet.",
        it: "Come l’adapter trova questa app.",
        ja: "アダプターがこのアプリを見つける仕組み。",
        zh: "适配器如何找到此应用。",
        ko: "어댑터가 이 앱을 찾는 방법이에요.",
        ru: "Как адаптер находит это приложение.",
    }
    /// OpenCode local connection: label of the local API row, and name of its check.
    opencode_local_api {
        en: "Local API",
        pt: "API local",
        es: "API local",
        fr: "API locale",
        de: "Lokale API",
        it: "API locale",
        ja: "ローカルAPI",
        zh: "本地 API",
        ko: "로컬 API",
        ru: "Локальный API",
    }
    /// OpenCode local connection: the API is inactive.
    opencode_inactive {
        en: "Inactive",
        pt: "Inativa",
        es: "Inactiva",
        fr: "Inactive",
        de: "Inaktiv",
        it: "Inattiva",
        ja: "無効",
        zh: "未启用",
        ko: "꺼짐",
        ru: "Неактивен",
    }
    /// OpenCode local connection: label of the capture contract row.
    opencode_contract {
        en: "Capture contract",
        pt: "Contrato de captura",
        es: "Contrato de captura",
        fr: "Contrat de capture",
        de: "Erfassungsvertrag",
        it: "Contratto di acquisizione",
        ja: "キャプチャの契約",
        zh: "捕获契约",
        ko: "캡처 계약",
        ru: "Контракт захвата",
    }
    /// OpenCode local connection: label of the data folder row.
    opencode_data_dir {
        en: "Data folder",
        pt: "Pasta de dados",
        es: "Carpeta de datos",
        fr: "Dossier de données",
        de: "Datenordner",
        it: "Cartella dei dati",
        ja: "データフォルダー",
        zh: "数据文件夹",
        ko: "데이터 폴더",
        ru: "Папка данных",
    }
    /// OpenCode check outcome: OK.
    opencode_check_ok {
        en: "OK",
        pt: "OK",
        es: "OK",
        fr: "OK",
        de: "OK",
        it: "OK",
        ja: "OK",
        zh: "OK",
        ko: "OK",
        ru: "OK",
    }
    /// OpenCode check outcome: needs attention.
    opencode_check_warning {
        en: "Attention",
        pt: "Atenção",
        es: "Atención",
        fr: "Attention",
        de: "Hinweis",
        it: "Attenzione",
        ja: "注意",
        zh: "注意",
        ko: "주의",
        ru: "Внимание",
    }
    /// OpenCode check outcome: failed.
    opencode_check_failed {
        en: "Failed",
        pt: "Falha",
        es: "Fallo",
        fr: "Échec",
        de: "Fehler",
        it: "Errore",
        ja: "失敗",
        zh: "失败",
        ko: "실패",
        ru: "Сбой",
    }
    /// OpenCode check outcome: does not apply.
    opencode_check_skipped {
        en: "Not applicable",
        pt: "Não se aplica",
        es: "No aplica",
        fr: "Sans objet",
        de: "Nicht zutreffend",
        it: "Non applicabile",
        ja: "対象外",
        zh: "不适用",
        ko: "해당 없음",
        ru: "Неприменимо",
    }
    /// OpenCode check name: discovery file.
    opencode_check_discovery {
        en: "Discovery file",
        pt: "Arquivo de descoberta",
        es: "Archivo de descubrimiento",
        fr: "Fichier de découverte",
        de: "Discovery-Datei",
        it: "File di discovery",
        ja: "ディスカバリーファイル",
        zh: "发现文件",
        ko: "디스커버리 파일",
        ru: "Файл обнаружения",
    }
    /// OpenCode check name: session token.
    opencode_check_token {
        en: "Session token",
        pt: "Token de sessão",
        es: "Token de sesión",
        fr: "Jeton de session",
        de: "Sitzungstoken",
        it: "Token di sessione",
        ja: "セッショントークン",
        zh: "会话令牌",
        ko: "세션 토큰",
        ru: "Токен сессии",
    }
    /// OpenCode check name: outbox.
    opencode_check_outbox {
        en: "Outbox",
        pt: "Outbox",
        es: "Outbox",
        fr: "Outbox",
        de: "Outbox",
        it: "Outbox",
        ja: "アウトボックス",
        zh: "发件箱",
        ko: "아웃박스",
        ru: "Outbox",
    }
    /// OpenCode check name: captures received.
    opencode_check_captures {
        en: "Captures received",
        pt: "Capturas recebidas",
        es: "Capturas recibidas",
        fr: "Captures reçues",
        de: "Empfangene Erfassungen",
        it: "Acquisizioni ricevute",
        ja: "受信したキャプチャ",
        zh: "已收到的捕获",
        ko: "받은 캡처",
        ru: "Полученные захваты",
    }
    /// Provider option: Claude Code, experimental.
    ai_kind_claude_code_title {
        en: "Claude Code (experimental)",
        pt: "Claude Code (experimental)",
        es: "Claude Code (experimental)",
        fr: "Claude Code (expérimental)",
        de: "Claude Code (experimentell)",
        it: "Claude Code (sperimentale)",
        ja: "Claude Code（試験的）",
        zh: "Claude Code（实验性）",
        ko: "Claude Code (실험적)",
        ru: "Claude Code (экспериментально)",
    }
    /// Provider option: what Claude Code does.
    ai_kind_claude_code_body {
        en: "Uses the Claude Code already signed in on this machine, no API key.",
        pt: "Usa o Claude Code já conectado nesta máquina, sem chave de API.",
        es: "Usa el Claude Code ya conectado en este equipo, sin clave de API.",
        fr: "Utilise le Claude Code déjà connecté sur cette machine, sans clé d’API.",
        de: "Nutzt das bereits angemeldete Claude Code auf diesem Rechner, ohne API-Schlüssel.",
        it: "Usa il Claude Code già connesso su questo computer, senza chiave API.",
        ja: "このマシンでログイン済みのClaude Codeを使います。APIキーは不要です。",
        zh: "使用本机已登录的 Claude Code，无需 API 密钥。",
        ko: "이 컴퓨터에 이미 로그인된 Claude Code를 사용해요. API 키는 필요 없어요.",
        ru: "Использует Claude Code, уже выполнивший вход на этом компьютере, без API-ключа.",
    }
    /// Hint: Claude Code needs a model before saving.
    ai_hint_pick_claude_code_model {
        en: "Pick a Claude Code model to save.",
        pt: "Escolha um modelo do Claude Code para salvar.",
        es: "Elige un modelo de Claude Code para guardar.",
        fr: "Choisissez un modèle Claude Code pour enregistrer.",
        de: "Wähle ein Claude-Code-Modell, um zu speichern.",
        it: "Scegli un modello di Claude Code per salvare.",
        ja: "保存するには、Claude Codeのモデルを選んでください。",
        zh: "请选择 Claude Code 模型后再保存。",
        ko: "저장하려면 Claude Code 모델을 고르세요.",
        ru: "Выберите модель Claude Code, чтобы сохранить.",
    }
    /// Diagnostics: task kind, search terms.
    diag_kind_search_terms {
        en: "Search terms",
        pt: "Termos de busca",
        es: "Términos de búsqueda",
        fr: "Termes de recherche",
        de: "Suchbegriffe",
        it: "Termini di ricerca",
        ja: "検索語",
        zh: "搜索词",
        ko: "검색어",
        ru: "Поисковые термины",
    }
    /// Diagnostics: task kind, links of a decision to the Map.
    diag_kind_links {
        en: "Map links",
        pt: "Vínculos com o Mapa",
        es: "Vínculos con el Mapa",
        fr: "Liens avec la Carte",
        de: "Verknüpfungen zur Karte",
        it: "Collegamenti alla Mappa",
        ja: "マップへのリンク",
        zh: "地图链接",
        ko: "맵 연결",
        ru: "Связи с картой",
    }
    /// Consent checklist: Claude Code signed in.
    step_claude_connected_title {
        en: "Claude Code connected",
        pt: "Claude Code conectado",
        es: "Claude Code conectado",
        fr: "Claude Code connecté",
        de: "Claude Code verbunden",
        it: "Claude Code collegato",
        ja: "Claude Code接続済み",
        zh: "Claude Code 已连接",
        ko: "Claude Code 연결됨",
        ru: "Claude Code подключён",
    }
    /// Consent checklist: where the Claude Code login happens.
    step_claude_connected_hint {
        en: "Signed in on the terminal",
        pt: "Login feito no terminal",
        es: "Sesión iniciada en el terminal",
        fr: "Connexion faite dans le terminal",
        de: "Im Terminal angemeldet",
        it: "Accesso fatto nel terminale",
        ja: "ターミナルでログイン済み",
        zh: "已在终端登录",
        ko: "터미널에서 로그인함",
        ru: "Вход выполнен в терминале",
    }
    /// Preview destination: Claude Code sends to Anthropic.
    destination_claude_code {
        en: "Anthropic, via Claude Code",
        pt: "Anthropic, pelo Claude Code",
        es: "Anthropic, mediante Claude Code",
        fr: "Anthropic, via Claude Code",
        de: "Anthropic, über Claude Code",
        it: "Anthropic, tramite Claude Code",
        ja: "Anthropic（Claude Code経由）",
        zh: "Anthropic（通过 Claude Code）",
        ko: "Anthropic (Claude Code 경유)",
        ru: "Anthropic, через Claude Code",
    }
    /// Preview note: Claude Code sends the excerpts with its own login.
    destination_claude_code_note {
        en: "The Claude Code on this machine sends the excerpts with its own login.",
        pt: "O Claude Code desta máquina envia os trechos com o login dele.",
        es: "El Claude Code de este equipo envía los fragmentos con su propio inicio de sesión.",
        fr: "Le Claude Code de cette machine envoie les extraits avec sa propre connexion.",
        de: "Das Claude Code auf diesem Rechner sendet die Auszüge mit seiner eigenen Anmeldung.",
        it: "Il Claude Code di questo computer invia i frammenti con il proprio accesso.",
        ja: "このマシンのClaude Codeが、自身のログインで抜粋を送信します。",
        zh: "本机的 Claude Code 会用它自己的登录发送这些片段。",
        ko: "이 컴퓨터의 Claude Code가 자체 로그인으로 발췌문을 보내요.",
        ru: "Claude Code на этом компьютере отправляет фрагменты со своим входом.",
    }
    /// Claude Code card: no probe available.
    claude_check_unavailable {
        en: "The Claude Code check is not available here.",
        pt: "A verificação do Claude Code não está disponível aqui.",
        es: "La comprobación de Claude Code no está disponible aquí.",
        fr: "La vérification de Claude Code n’est pas disponible ici.",
        de: "Die Claude-Code-Prüfung ist hier nicht verfügbar.",
        it: "La verifica di Claude Code non è disponibile qui.",
        ja: "ここではClaude Codeの確認を利用できません。",
        zh: "此处无法检查 Claude Code。",
        ko: "여기서는 Claude Code 확인을 사용할 수 없어요.",
        ru: "Проверка Claude Code здесь недоступна.",
    }
    /// Claude Code card: pill, executable not found.
    claude_not_found {
        en: "Not found",
        pt: "Não encontrado",
        es: "No encontrado",
        fr: "Introuvable",
        de: "Nicht gefunden",
        it: "Non trovato",
        ja: "見つかりません",
        zh: "未找到",
        ko: "찾을 수 없음",
        ru: "Не найден",
    }
    /// Claude Code card: pill, check failed.
    claude_unavailable {
        en: "Unavailable",
        pt: "Indisponível",
        es: "No disponible",
        fr: "Indisponible",
        de: "Nicht verfügbar",
        it: "Non disponibile",
        ja: "利用不可",
        zh: "不可用",
        ko: "사용할 수 없음",
        ru: "Недоступно",
    }
    /// Claude Code card: pill, not signed in.
    claude_signed_out {
        en: "Signed out",
        pt: "Sem login",
        es: "Sin sesión",
        fr: "Non connecté",
        de: "Nicht angemeldet",
        it: "Non connesso",
        ja: "未ログイン",
        zh: "未登录",
        ko: "로그인 안 됨",
        ru: "Вход не выполнен",
    }
    /// Claude Code card: pill, signed in.
    claude_connected {
        en: "Connected",
        pt: "Conectado",
        es: "Conectado",
        fr: "Connecté",
        de: "Verbunden",
        it: "Connesso",
        ja: "接続済み",
        zh: "已连接",
        ko: "연결됨",
        ru: "Подключено",
    }
    /// Claude Code card: how to install it. XEMNAS_CLAUDE_CODE and PATH stay as is.
    claude_install {
        en: "Install Claude Code and sign in to it in a terminal. If it is not on the PATH, point XEMNAS_CLAUDE_CODE to the executable.",
        pt: "Instale o Claude Code e entre nele num terminal. Fora do PATH, aponte XEMNAS_CLAUDE_CODE para o executável.",
        es: "Instala Claude Code e inicia sesión en un terminal. Si no está en el PATH, apunta XEMNAS_CLAUDE_CODE al ejecutable.",
        fr: "Installez Claude Code et connectez-vous dans un terminal. S’il n’est pas dans le PATH, faites pointer XEMNAS_CLAUDE_CODE vers l’exécutable.",
        de: "Installiere Claude Code und melde dich in einem Terminal an. Liegt es nicht im PATH, setze XEMNAS_CLAUDE_CODE auf die ausführbare Datei.",
        it: "Installa Claude Code e accedi in un terminale. Se non è nel PATH, punta XEMNAS_CLAUDE_CODE all’eseguibile.",
        ja: "Claude Codeをインストールし、ターミナルでログインしてください。PATHにない場合は、XEMNAS_CLAUDE_CODEに実行ファイルを指定します。",
        zh: "请安装 Claude Code 并在终端中登录。若不在 PATH 中，请将 XEMNAS_CLAUDE_CODE 指向可执行文件。",
        ko: "Claude Code를 설치하고 터미널에서 로그인하세요. PATH에 없다면 XEMNAS_CLAUDE_CODE가 실행 파일을 가리키게 하세요.",
        ru: "Установите Claude Code и войдите в него в терминале. Если его нет в PATH, укажите в XEMNAS_CLAUDE_CODE путь к исполняемому файлу.",
    }
    /// Claude Code card: how to sign in.
    claude_login {
        en: "Sign in to Claude Code in a terminal with the command below, then check again. The password never goes through xemnas.",
        pt: "Entre no Claude Code num terminal com o comando abaixo e verifique de novo. A senha nunca passa pela xemnas.",
        es: "Inicia sesión en Claude Code en un terminal con el comando de abajo y vuelve a comprobar. La contraseña nunca pasa por xemnas.",
        fr: "Connectez-vous à Claude Code dans un terminal avec la commande ci-dessous, puis vérifiez à nouveau. Le mot de passe ne passe jamais par xemnas.",
        de: "Melde dich in einem Terminal mit dem Befehl unten bei Claude Code an und prüfe erneut. Das Passwort läuft nie über xemnas.",
        it: "Accedi a Claude Code in un terminale con il comando qui sotto e verifica di nuovo. La password non passa mai da xemnas.",
        ja: "下のコマンドでターミナルからClaude Codeにログインし、もう一度確認してください。パスワードがxemnasを通ることはありません。",
        zh: "在终端中用下面的命令登录 Claude Code，然后重新检查。密码绝不会经过 xemnas。",
        ko: "아래 명령으로 터미널에서 Claude Code에 로그인한 뒤 다시 확인하세요. 비밀번호는 xemnas를 거치지 않아요.",
        ru: "Войдите в Claude Code в терминале командой ниже и проверьте снова. Пароль никогда не проходит через xemnas.",
    }
    /// Claude Code card: experimental notice.
    claude_experimental {
        en: "Experimental. Each analysis counts toward your plan usage, together with Claude Code itself.",
        pt: "Experimental. Cada análise conta no uso do seu plano, junto com o próprio Claude Code.",
        es: "Experimental. Cada análisis cuenta en el uso de tu plan, junto con el propio Claude Code.",
        fr: "Expérimental. Chaque analyse compte dans l’utilisation de votre forfait, avec Claude Code lui-même.",
        de: "Experimentell. Jede Analyse zählt zur Nutzung deines Tarifs, zusammen mit Claude Code selbst.",
        it: "Sperimentale. Ogni analisi conta nell’uso del tuo piano, insieme a Claude Code stesso.",
        ja: "試験的な機能です。分析のたびに、Claude Code自体の利用分とあわせてプランの使用量に加算されます。",
        zh: "实验性功能。每次分析都会计入你的套餐用量，与 Claude Code 本身的用量合并计算。",
        ko: "실험 기능이에요. 분석할 때마다 Claude Code 자체 사용량과 함께 플랜 사용량에 포함돼요.",
        ru: "Экспериментально. Каждый анализ учитывается в использовании вашего тарифа вместе с самим Claude Code.",
    }
    /// Claude Code card: what it is.
    claude_card_body {
        en: "xemnas uses the Claude Code on this machine; the login stays with it, never with xemnas.",
        pt: "A xemnas usa o Claude Code desta máquina; o login fica com ele, nunca com a xemnas.",
        es: "xemnas usa el Claude Code de este equipo; el inicio de sesión queda con él, nunca con xemnas.",
        fr: "xemnas utilise le Claude Code de cette machine ; la connexion reste chez lui, jamais chez xemnas.",
        de: "xemnas nutzt das Claude Code auf diesem Rechner; die Anmeldung bleibt dort, nie bei xemnas.",
        it: "xemnas usa il Claude Code di questo computer; l’accesso resta a lui, mai a xemnas.",
        ja: "xemnasはこのマシンのClaude Codeを使います。ログイン情報はClaude Codeが持ち、xemnasには渡りません。",
        zh: "xemnas 使用本机的 Claude Code；登录信息由它保管，绝不交给 xemnas。",
        ko: "xemnas는 이 컴퓨터의 Claude Code를 사용해요. 로그인은 Claude Code가 갖고, xemnas는 절대 갖지 않아요.",
        ru: "xemnas использует Claude Code на этом компьютере; вход остаётся у него, но не у xemnas.",
    }
    /// Claude Code card: sign-in method claude.ai.
    claude_ai_account {
        en: "claude.ai account",
        pt: "conta claude.ai",
        es: "cuenta de claude.ai",
        fr: "compte claude.ai",
        de: "claude.ai-Konto",
        it: "account claude.ai",
        ja: "claude.aiアカウント",
        zh: "claude.ai 账户",
        ko: "claude.ai 계정",
        ru: "аккаунт claude.ai",
    }
    /// Claude Code card: account without e-mail.
    claude_account_connected {
        en: "Account connected",
        pt: "Conta conectada",
        es: "Cuenta conectada",
        fr: "Compte connecté",
        de: "Konto verbunden",
        it: "Account collegato",
        ja: "アカウント接続済み",
        zh: "账户已连接",
        ko: "계정 연결됨",
        ru: "Аккаунт подключён",
    }
    /// Claude Code card: check in progress.
    claude_checking {
        en: "Checking…",
        pt: "Verificando…",
        es: "Comprobando…",
        fr: "Vérification…",
        de: "Prüfe …",
        it: "Verifica in corso…",
        ja: "確認中…",
        zh: "检查中…",
        ko: "확인 중…",
        ru: "Проверяю…",
    }
    /// Claude Code card: check button.
    claude_check_again {
        en: "Check again",
        pt: "Verificar de novo",
        es: "Comprobar de nuevo",
        fr: "Vérifier à nouveau",
        de: "Erneut prüfen",
        it: "Verifica di nuovo",
        ja: "もう一度確認",
        zh: "重新检查",
        ko: "다시 확인",
        ru: "Проверить снова",
    }
    /// Claude Code card: copy the login command.
    claude_copy_command {
        en: "Copy command",
        pt: "Copiar comando",
        es: "Copiar comando",
        fr: "Copier la commande",
        de: "Befehl kopieren",
        it: "Copia comando",
        ja: "コマンドをコピー",
        zh: "复制命令",
        ko: "명령 복사",
        ru: "Скопировать команду",
    }
    /// Toast: login command copied.
    claude_command_copied {
        en: "Command copied.",
        pt: "Comando copiado.",
        es: "Comando copiado.",
        fr: "Commande copiée.",
        de: "Befehl kopiert.",
        it: "Comando copiato.",
        ja: "コマンドをコピーしました。",
        zh: "命令已复制。",
        ko: "명령을 복사했어요.",
        ru: "Команда скопирована.",
    }
}

formats! {
    /// Status line body: who analyses the captures.
    ai_status_active_body(who: &str) {
        en: "Captures are analyzed {who}, within the limits of the preview.",
        pt: "Capturas são analisadas {who}, dentro dos limites da prévia.",
        es: "Las capturas se analizan {who}, dentro de los límites de la vista previa.",
        fr: "Les captures sont analysées {who}, dans les limites de l’aperçu.",
        de: "Erfassungen werden {who} analysiert, innerhalb der Grenzen der Vorschau.",
        it: "Le acquisizioni vengono analizzate {who}, entro i limiti dell’anteprima.",
        ja: "キャプチャは{who}で分析されます。プレビューの範囲内に限られます。",
        zh: "捕获由{who}分析，范围不超出预览。",
        ko: "캡처는 {who} 분석돼요. 미리 보기의 범위 안에서만 보내요.",
        ru: "Захваты анализируются {who} в пределах предпросмотра.",
    }
    /// Status line body: why it is blocked.
    ai_status_blocked_body(reason: &str) {
        en: "{reason} Until then, no capture is sent or analyzed.",
        pt: "{reason} Até lá, nenhuma captura é enviada nem analisada.",
        es: "{reason} Hasta entonces, no se envía ni se analiza ninguna captura.",
        fr: "{reason} D’ici là, aucune capture n’est envoyée ni analysée.",
        de: "{reason} Bis dahin wird keine Erfassung gesendet oder analysiert.",
        it: "{reason} Fino ad allora, nessuna acquisizione viene inviata né analizzata.",
        ja: "{reason}それまでは、キャプチャの送信も分析も行われません。",
        zh: "{reason}在此之前，不会发送或分析任何捕获。",
        ko: "{reason} 그때까지는 어떤 캡처도 전송하거나 분석하지 않아요.",
        ru: "{reason} До тех пор ни один захват не отправляется и не анализируется.",
    }
    /// Preview value: approximate characters per analysis.
    ai_preview_chars(chars: &str) {
        en: "≈ {chars} characters",
        pt: "≈ {chars} caracteres",
        es: "≈ {chars} caracteres",
        fr: "≈ {chars} caractères",
        de: "≈ {chars} Zeichen",
        it: "≈ {chars} caratteri",
        ja: "約 {chars} 文字",
        zh: "约 {chars} 个字符",
        ko: "약 {chars}자",
        ru: "≈ {chars} символов",
    }
    /// Preview row: character cap of one category per item.
    ai_preview_up_to(chars: &str) {
        en: "up to {chars} per item",
        pt: "até {chars} por item",
        es: "hasta {chars} por elemento",
        fr: "jusqu’à {chars} par élément",
        de: "bis zu {chars} pro Element",
        it: "fino a {chars} per elemento",
        ja: "項目ごとに最大 {chars}",
        zh: "每项最多 {chars}",
        ko: "항목당 최대 {chars}",
        ru: "до {chars} на элемент",
    }
    /// Consent card: consent is active, with its date.
    ai_consent_given(granted: &str) {
        en: "Consented on {granted} to the preview above. Changing the configuration requires \
            consenting again.",
        pt: "Consentido em {granted} para a prévia acima. Mudar a configuração exige consentir \
            de novo.",
        es: "Consentido el {granted} para la vista previa de arriba. Cambiar la configuración \
            exige volver a consentir.",
        fr: "Consentement donné le {granted} pour l’aperçu ci-dessus. Modifier la configuration \
            exige de consentir de nouveau.",
        de: "Eingewilligt am {granted} für die Vorschau oben. Eine Änderung der Konfiguration \
            erfordert erneute Einwilligung.",
        it: "Consenso dato il {granted} per l’anteprima qui sopra. Cambiare la configurazione \
            richiede un nuovo consenso.",
        ja: "{granted}に上のプレビューへ同意しました。設定を変えると、もう一度同意が必要です。",
        zh: "已于 {granted} 同意上方的预览。更改配置后需要重新同意。",
        ko: "{granted}에 위 미리 보기에 동의했어요. 설정을 바꾸면 다시 동의해야 해요.",
        ru: "Согласие на предпросмотр выше дано {granted}. После смены настройки нужно дать \
            согласие снова.",
    }
    /// Diagnostics toast: the document was saved.
    diag_saved_at(path: &str) {
        en: "Diagnostics saved to {path}",
        pt: "Diagnóstico salvo em {path}",
        es: "Diagnóstico guardado en {path}",
        fr: "Diagnostic enregistré dans {path}",
        de: "Diagnose gespeichert unter {path}",
        it: "Diagnostica salvata in {path}",
        ja: "診断を {path} に保存しました",
        zh: "诊断已保存到 {path}",
        ko: "진단을 {path}에 저장했어요",
        ru: "Диагностика сохранена: {path}",
    }
    /// Diagnostics: medians, p95 and context counts.
    diag_metrics_note(capture: &str, review: &str, decided: i64, sent: i64, measured: i64) {
        en: "Medians with p95 of {capture} and {review} · {decided} candidates decided · \
            context: {sent} sent, {measured} measured only",
        pt: "Medianas com p95 de {capture} e {review} · {decided} candidatos decididos · \
            contexto: {sent} enviados, {measured} só medidos",
        es: "Medianas con p95 de {capture} y {review} · {decided} candidatos decididos · \
            contexto: {sent} enviados, {measured} solo medidos",
        fr: "Médianes avec p95 de {capture} et {review} · {decided} candidats décidés · \
            contexte : {sent} envoyés, {measured} mesurés seulement",
        de: "Mediane mit p95 von {capture} und {review} · {decided} Kandidaten entschieden · \
            Kontext: {sent} gesendet, {measured} nur gemessen",
        it: "Mediane con p95 di {capture} e {review} · {decided} candidati decisi · contesto: \
            {sent} inviati, {measured} solo misurati",
        ja: "中央値とp95は{capture}と{review} · 決定済みの候補 {decided}件 · コンテキスト：送信 {sent}件、計測のみ {measured}件",
        zh: "中位数及 p95 分别为 {capture} 和 {review} · 已决定候选 {decided} 个 · 上下文：已发送 {sent} 个，仅测量 \
            {measured} 个",
        ko: "중앙값과 p95는 {capture}, {review} · 결정된 후보 {decided}개 · 컨텍스트: 전송 {sent}개, 측정만 {measured}개",
        ru: "Медианы с p95 {capture} и {review} · решено кандидатов: {decided} · контекст: \
            отправлено {sent}, только измерено {measured}",
    }
    /// Diagnostics: verdict, not enough decisions to know.
    diag_calib_too_few(decided: i64, min: usize) {
        en: "Can’t tell yet: {decided} of {min} decisions, and you need to have accepted and \
            dismissed at least one.",
        pt: "Ainda não dá para saber: {decided} de {min} decisões, e é preciso ter aceitado e \
            descartado pelo menos uma.",
        es: "Aún no se puede saber: {decided} de {min} decisiones, y hay que haber aceptado y \
            descartado al menos una.",
        fr: "Impossible de le savoir pour l’instant : {decided} décisions sur {min}, et il faut \
            en avoir accepté et écarté au moins une.",
        de: "Noch nicht absehbar: {decided} von {min} Entscheidungen, und du musst mindestens \
            eine angenommen und eine verworfen haben.",
        it: "Ancora impossibile dirlo: {decided} decisioni su {min}, e devi averne accettata e \
            scartata almeno una.",
        ja: "まだ判断できません：決定は{min}件中{decided}件で、承認と破棄をそれぞれ1件以上行う必要があります。",
        zh: "暂时无法判断：{min} 个决策中已有 {decided} 个，并且需要至少各接受和丢弃过一个。",
        ko: "아직 알 수 없어요. 결정 {decided}/{min}개이고, 수락과 폐기를 각각 한 번 이상 해야 해요.",
        ru: "Пока рано судить: {decided} из {min} решений, и нужно хотя бы одно принять и одно \
            отбросить.",
    }
    /// Diagnostics: share kept in a confidence band, with its decision count.
    diag_bin_label(kept: f32, decisions: &str) {
        en: "{kept:.0}% kept · {decisions}",
        pt: "{kept:.0}% mantidas · {decisions}",
        es: "{kept:.0}% conservadas · {decisions}",
        fr: "{kept:.0}% conservées · {decisions}",
        de: "{kept:.0}% behalten · {decisions}",
        it: "{kept:.0}% mantenute · {decisions}",
        ja: "維持 {kept:.0}% · {decisions}",
        zh: "保留 {kept:.0}% · {decisions}",
        ko: "유지 {kept:.0}% · {decisions}",
        ru: "сохранено {kept:.0}% · {decisions}",
    }
    /// Diagnostics: separation figure and decision counts.
    diag_separation(value: f64, predicts: f64, accepted: i64, edited: i64, dismissed: i64) {
        en: "Separation {value:.2} (0.50 is chance; {predicts:.2} or more predicts) · \
            {accepted} accepted, {edited} edited, {dismissed} dismissed",
        pt: "Separação {value:.2} (0,50 é acaso; {predicts:.2} ou mais prevê) · {accepted} \
            aceitas, {edited} editadas, {dismissed} descartadas",
        es: "Separación {value:.2} (0,50 es azar; {predicts:.2} o más predice) · {accepted} \
            aceptadas, {edited} editadas, {dismissed} descartadas",
        fr: "Séparation {value:.2} (0,50 correspond au hasard ; {predicts:.2} ou plus prédit) · \
            {accepted} acceptées, {edited} modifiées, {dismissed} écartées",
        de: "Trennschärfe {value:.2} (0,50 ist Zufall; ab {predicts:.2} sagt sie voraus) · \
            {accepted} angenommen, {edited} bearbeitet, {dismissed} verworfen",
        it: "Separazione {value:.2} (0,50 è il caso; da {predicts:.2} in su prevede) · \
            {accepted} accettate, {edited} modificate, {dismissed} scartate",
        ja: "分離度 {value:.2}（0.50は偶然、{predicts:.2}以上で予測可能） · 承認 {accepted}件、編集 {edited}件、破棄 \
            {dismissed}件",
        zh: "区分度 {value:.2}（0.50 为随机；达到 {predicts:.2} 及以上即可预测） · 已接受 {accepted}、已编辑 \
            {edited}、已丢弃 {dismissed}",
        ko: "분리도 {value:.2} (0.50은 우연, {predicts:.2} 이상이면 예측 가능) · 수락 {accepted}개, 편집 \
            {edited}개, 폐기 {dismissed}개",
        ru: "Разделение {value:.2} (0,50 — случайность; от {predicts:.2} уже предсказывает) · \
            принято {accepted}, изменено {edited}, отброшено {dismissed}",
    }
    /// Diagnostics: counts of one queue.
    diag_lane_counts(queued: usize, running: usize, failed: usize) {
        en: "{queued} queued · {running} running · {failed} failed",
        pt: "{queued} na fila · {running} executando · {failed} com falha",
        es: "{queued} en cola · {running} en ejecución · {failed} con error",
        fr: "{queued} en attente · {running} en cours · {failed} en échec",
        de: "{queued} in der Warteschlange · {running} laufend · {failed} fehlgeschlagen",
        it: "{queued} in coda · {running} in esecuzione · {failed} non riuscite",
        ja: "待機中 {queued}件 · 実行中 {running}件 · 失敗 {failed}件",
        zh: "排队 {queued} · 运行中 {running} · 失败 {failed}",
        ko: "대기 {queued}개 · 실행 중 {running}개 · 실패 {failed}개",
        ru: "в очереди: {queued} · выполняется: {running} · с ошибкой: {failed}",
    }
    /// Diagnostics: date, attempts and error code of a task.
    diag_job_meta(date: &str, attempts: i64, code: &str) {
        en: "{date} · {attempts} attempt(s){code}",
        pt: "{date} · {attempts} tentativa(s){code}",
        es: "{date} · {attempts} intento(s){code}",
        fr: "{date} · {attempts} tentative(s){code}",
        de: "{date} · {attempts} Versuch(e){code}",
        it: "{date} · tentativi: {attempts}{code}",
        ja: "{date} · 試行 {attempts}回{code}",
        zh: "{date} · 尝试 {attempts} 次{code}",
        ko: "{date} · {attempts}회 시도{code}",
        ru: "{date} · попыток: {attempts}{code}",
    }
    /// Diagnostics: error code appended to the task line.
    diag_job_code(code: &str) {
        en: " · code {code}",
        pt: " · código {code}",
        es: " · código {code}",
        fr: " · code {code}",
        de: " · Code {code}",
        it: " · codice {code}",
        ja: " · コード {code}",
        zh: " · 代码 {code}",
        ko: " · 코드 {code}",
        ru: " · код {code}",
    }
    /// Diagnostics: app version and database schema version.
    diag_about_version_value(app: &str, db: i64) {
        en: "{app} · database v{db}",
        pt: "{app} · banco v{db}",
        es: "{app} · base de datos v{db}",
        fr: "{app} · base de données v{db}",
        de: "{app} · Datenbank v{db}",
        it: "{app} · database v{db}",
        ja: "{app} · データベース v{db}",
        zh: "{app} · 数据库 v{db}",
        ko: "{app} · 데이터베이스 v{db}",
        ru: "{app} · база данных v{db}",
    }
    /// Diagnostics: extraction runs at an external host.
    diag_extraction_external(host: &str, mode: &str) {
        en: "external via {host} ({mode})",
        pt: "externa via {host} ({mode})",
        es: "externa vía {host} ({mode})",
        fr: "externe via {host} ({mode})",
        de: "extern über {host} ({mode})",
        it: "esterna tramite {host} ({mode})",
        ja: "{host} 経由の外部 ({mode})",
        zh: "通过 {host} 的外部提取（{mode}）",
        ko: "외부: {host} 경유 ({mode})",
        ru: "внешнее через {host} ({mode})",
    }
    /// Diagnostics: extraction runs locally.
    diag_extraction_local(mode: &str) {
        en: "local ({mode})",
        pt: "local ({mode})",
        es: "local ({mode})",
        fr: "locale ({mode})",
        de: "lokal ({mode})",
        it: "locale ({mode})",
        ja: "ローカル ({mode})",
        zh: "本地（{mode}）",
        ko: "로컬 ({mode})",
        ru: "локальное ({mode})",
    }
    /// Status line: captures are analyzed by a host.
    analysed_by_host(host: &str) {
        en: "by {host}",
        pt: "por {host}",
        es: "por {host}",
        fr: "par {host}",
        de: "von {host}",
        it: "da {host}",
        ja: "{host}",
        zh: "通过 {host}",
        ko: "{host}에서",
        ru: "через {host}",
    }
    /// Status line: captures are analyzed by the ChatGPT plan.
    analysed_by_chatgpt(model: &str) {
        en: "through your ChatGPT plan ({model})",
        pt: "pelo seu plano do ChatGPT ({model})",
        es: "con tu plan de ChatGPT ({model})",
        fr: "via votre forfait ChatGPT ({model})",
        de: "über deinen ChatGPT-Tarif ({model})",
        it: "con il tuo piano ChatGPT ({model})",
        ja: "ChatGPTのプラン（{model}）",
        zh: "通过你的 ChatGPT 套餐（{model}）",
        ko: "ChatGPT 플랜({model})으로",
        ru: "по вашему тарифу ChatGPT ({model})",
    }
    /// Status line: captures are analyzed by OpenCode.
    analysed_by_opencode(plan: &str, model: &str) {
        en: "by {plan} with {model}",
        pt: "pelo {plan} com {model}",
        es: "con {plan} usando {model}",
        fr: "par {plan} avec {model}",
        de: "von {plan} mit {model}",
        it: "da {plan} con {model}",
        ja: "{plan}（{model}）",
        zh: "通过 {plan}，使用 {model}",
        ko: "{plan}에서 {model}로",
        ru: "через {plan} с моделью {model}",
    }
    /// Provider form: the model list failed; the model can still be typed.
    models_failed(message: &str) {
        en: "{message} You can still type the model.",
        pt: "{message} Você ainda pode digitar o modelo.",
        es: "{message} Aún puedes escribir el modelo.",
        fr: "{message} Vous pouvez toujours saisir le modèle.",
        de: "{message} Du kannst das Modell trotzdem eintippen.",
        it: "{message} Puoi comunque digitare il modello.",
        ja: "{message}モデル名は手入力もできます。",
        zh: "{message}你仍然可以手动输入模型。",
        ko: "{message} 모델 이름을 직접 입력할 수도 있어요.",
        ru: "{message} Название модели можно ввести вручную.",
    }
    /// OpenCode toast: stalled captures went back to the queue.
    opencode_retried(moved: usize) {
        en: "{moved} capture(s) went back to the queue and will be imported the next time the \
            app opens.",
        pt: "{moved} captura(s) voltaram para a fila e serão importadas na próxima abertura.",
        es: "{moved} captura(s) volvieron a la cola y se importarán la próxima vez que se abra \
            la app.",
        fr: "{moved} capture(s) sont retournées dans la file et seront importées à la prochaine \
            ouverture de l’app.",
        de: "{moved} Erfassung(en) sind zurück in der Warteschlange und werden beim nächsten \
            Öffnen der App importiert.",
        it: "{moved} acquisizione/i sono tornate in coda e verranno importate alla prossima \
            apertura dell’app.",
        ja: "{moved}件のキャプチャをキューに戻しました。次回アプリを開いたときに取り込まれます。",
        zh: "{moved} 个捕获已回到队列，将在下次打开应用时导入。",
        ko: "캡처 {moved}개를 대기열로 되돌렸어요. 다음에 앱을 열 때 가져와요.",
        ru: "Захватов, вернувшихся в очередь: {moved}. Они будут импортированы при следующем \
            открытии приложения.",
    }
    /// OpenCode status: when the latest capture arrived.
    opencode_receiving_body(date: &str) {
        en: "OpenCode is sending captures to this app. The latest arrived on {date}.",
        pt: "O OpenCode está enviando capturas para este app. A última chegou em {date}.",
        es: "OpenCode está enviando capturas a esta app. La última llegó el {date}.",
        fr: "OpenCode envoie des captures à cette app. La dernière est arrivée le {date}.",
        de: "OpenCode sendet Erfassungen an diese App. Die letzte kam am {date} an.",
        it: "OpenCode sta inviando acquisizioni a questa app. L’ultima è arrivata il {date}.",
        ja: "OpenCodeがこのアプリにキャプチャを送っています。最新のものは{date}に届きました。",
        zh: "OpenCode 正在向此应用发送捕获。最近一次到达于 {date}。",
        ko: "OpenCode가 이 앱으로 캡처를 보내고 있어요. 가장 최근 캡처는 {date}에 도착했어요.",
        ru: "OpenCode отправляет захваты в это приложение. Последний пришёл {date}.",
    }
    /// OpenCode connection test: how many checks passed.
    opencode_summary_ok(n: usize) {
        en: "{n} OK",
        pt: "{n} ok",
        es: "{n} correctas",
        fr: "{n} réussies",
        de: "{n} in Ordnung",
        it: "{n} riuscite",
        ja: "正常 {n}件",
        zh: "{n} 项正常",
        ko: "정상 {n}개",
        ru: "в порядке: {n}",
    }
    /// OpenCode connection test: how many checks need attention.
    opencode_summary_warnings(n: usize) {
        en: "{n} need attention",
        pt: "{n} com atenção",
        es: "{n} requieren atención",
        fr: "{n} à surveiller",
        de: "{n} mit Hinweis",
        it: "{n} da controllare",
        ja: "注意 {n}件",
        zh: "{n} 项需注意",
        ko: "주의 {n}개",
        ru: "требуют внимания: {n}",
    }
    /// OpenCode adapters card: sessions and last capture date.
    opencode_adapter_meta(sessions: &str, date: &str) {
        en: "{sessions} · last capture on {date}",
        pt: "{sessions} · última captura em {date}",
        es: "{sessions} · última captura el {date}",
        fr: "{sessions} · dernière capture le {date}",
        de: "{sessions} · letzte Erfassung am {date}",
        it: "{sessions} · ultima acquisizione il {date}",
        ja: "{sessions} · 最後のキャプチャ：{date}",
        zh: "{sessions} · 最近一次捕获：{date}",
        ko: "{sessions} · 마지막 캡처: {date}",
        ru: "{sessions} · последний захват: {date}",
    }
    /// OpenCode outbox: the folder does not exist yet.
    opencode_outbox_missing(path: &str) {
        en: "{path} (not created yet)",
        pt: "{path} (ainda não criada)",
        es: "{path} (aún no creada)",
        fr: "{path} (pas encore créé)",
        de: "{path} (noch nicht angelegt)",
        it: "{path} (non ancora creata)",
        ja: "{path}（まだ作成されていません）",
        zh: "{path}（尚未创建）",
        ko: "{path} (아직 만들어지지 않음)",
        ru: "{path} (ещё не создана)",
    }
    /// OpenCode local connection: address and protocol of the API.
    opencode_api_value(port: u16, protocol: u32) {
        en: "127.0.0.1:{port} · protocol v{protocol}",
        pt: "127.0.0.1:{port} · protocolo v{protocol}",
        es: "127.0.0.1:{port} · protocolo v{protocol}",
        fr: "127.0.0.1:{port} · protocole v{protocol}",
        de: "127.0.0.1:{port} · Protokoll v{protocol}",
        it: "127.0.0.1:{port} · protocollo v{protocol}",
        ja: "127.0.0.1:{port} · プロトコル v{protocol}",
        zh: "127.0.0.1:{port} · 协议 v{protocol}",
        ko: "127.0.0.1:{port} · 프로토콜 v{protocol}",
        ru: "127.0.0.1:{port} · протокол v{protocol}",
    }
    /// OpenCode check: message with the last time.
    opencode_check_message(message: &str, date: &str) {
        en: "{message} Last on {date}.",
        pt: "{message} Última em {date}.",
        es: "{message} Última el {date}.",
        fr: "{message} Dernière le {date}.",
        de: "{message} Zuletzt am {date}.",
        it: "{message} Ultima il {date}.",
        ja: "{message}最後：{date}。",
        zh: "{message}最近一次：{date}。",
        ko: "{message} 마지막: {date}.",
        ru: "{message} Последний раз: {date}.",
    }
    /// Status line: captures are analyzed by Claude Code.
    analysed_by_claude_code(model: &str) {
        en: "by the Claude Code on this machine ({model})",
        pt: "pelo Claude Code desta máquina ({model})",
        es: "con el Claude Code de este equipo ({model})",
        fr: "par le Claude Code de cette machine ({model})",
        de: "vom Claude Code auf diesem Rechner ({model})",
        it: "dal Claude Code di questo computer ({model})",
        ja: "このマシンのClaude Code（{model}）",
        zh: "通过本机的 Claude Code（{model}）",
        ko: "이 컴퓨터의 Claude Code({model})로",
        ru: "через Claude Code на этом компьютере ({model})",
    }
    /// Claude Code card: subscription plan name.
    claude_plan(plan: &str) {
        en: "{plan} plan",
        pt: "Plano {plan}",
        es: "Plan {plan}",
        fr: "Forfait {plan}",
        de: "Tarif {plan}",
        it: "Piano {plan}",
        ja: "{plan}プラン",
        zh: "{plan} 套餐",
        ko: "{plan} 플랜",
        ru: "Тариф {plan}",
    }
}

counted! {
    /// Count of decisions.
    decisions_count {
        en: ("{n} decision", "{n} decisions"),
        pt: ("{n} decisão", "{n} decisões"),
        es: ("{n} decisión", "{n} decisiones"),
        fr: ("{n} décision", "{n} décisions"),
        de: ("{n} Entscheidung", "{n} Entscheidungen"),
        it: ("{n} decisione", "{n} decisioni"),
        ja: "{n}件の決定",
        zh: "{n} 个决策",
        ko: "결정 {n}개",
        ru: ("{n} решение", "{n} решения", "{n} решений"),
    }
    /// Count of projects.
    projects_count {
        en: ("{n} project", "{n} projects"),
        pt: ("{n} projeto", "{n} projetos"),
        es: ("{n} proyecto", "{n} proyectos"),
        fr: ("{n} projet", "{n} projets"),
        de: ("{n} Projekt", "{n} Projekte"),
        it: ("{n} progetto", "{n} progetti"),
        ja: "{n}件のプロジェクト",
        zh: "{n} 个项目",
        ko: "프로젝트 {n}개",
        ru: ("{n} проект", "{n} проекта", "{n} проектов"),
    }
    /// Count of captures.
    captures_count {
        en: ("{n} capture", "{n} captures"),
        pt: ("{n} captura", "{n} capturas"),
        es: ("{n} captura", "{n} capturas"),
        fr: ("{n} capture", "{n} captures"),
        de: ("{n} Erfassung", "{n} Erfassungen"),
        it: ("{n} acquisizione", "{n} acquisizioni"),
        ja: "{n}件のキャプチャ",
        zh: "{n} 个捕获",
        ko: "캡처 {n}개",
        ru: ("{n} захват", "{n} захвата", "{n} захватов"),
    }
    /// Count of decision revisions.
    revisions_count {
        en: ("{n} revision", "{n} revisions"),
        pt: ("{n} revisão", "{n} revisões"),
        es: ("{n} revisión", "{n} revisiones"),
        fr: ("{n} révision", "{n} révisions"),
        de: ("{n} Revision", "{n} Revisionen"),
        it: ("{n} revisione", "{n} revisioni"),
        ja: "{n}件のリビジョン",
        zh: "{n} 个修订",
        ko: "리비전 {n}개",
        ru: ("{n} ревизия", "{n} ревизии", "{n} ревизий"),
    }
    /// Provider form: count of available models, in capitals.
    models_available_count {
        en: ("{n} MODEL AVAILABLE", "{n} MODELS AVAILABLE"),
        pt: ("{n} MODELO DISPONÍVEL", "{n} MODELOS DISPONÍVEIS"),
        es: ("{n} MODELO DISPONIBLE", "{n} MODELOS DISPONIBLES"),
        fr: ("{n} MODÈLE DISPONIBLE", "{n} MODÈLES DISPONIBLES"),
        de: ("{n} MODELL VERFÜGBAR", "{n} MODELLE VERFÜGBAR"),
        it: ("{n} MODELLO DISPONIBILE", "{n} MODELLI DISPONIBILI"),
        ja: "利用可能なモデル {n}件",
        zh: "{n} 个可用模型",
        ko: "사용 가능한 모델 {n}개",
        ru: ("ДОСТУПНА {n} МОДЕЛЬ", "ДОСТУПНО {n} МОДЕЛИ", "ДОСТУПНО {n} МОДЕЛЕЙ"),
    }
    /// OpenCode connection test: how many checks failed.
    opencode_summary_failures {
        en: ("{n} failure", "{n} failures"),
        pt: ("{n} falha", "{n} falhas"),
        es: ("{n} fallo", "{n} fallos"),
        fr: ("{n} échec", "{n} échecs"),
        de: ("{n} Fehler", "{n} Fehler"),
        it: ("{n} errore", "{n} errori"),
        ja: "失敗 {n}件",
        zh: "{n} 项失败",
        ko: "실패 {n}개",
        ru: ("{n} сбой", "{n} сбоя", "{n} сбоев"),
    }
    /// Count of sessions.
    sessions_count {
        en: ("{n} session", "{n} sessions"),
        pt: ("{n} sessão", "{n} sessões"),
        es: ("{n} sesión", "{n} sesiones"),
        fr: ("{n} session", "{n} sessions"),
        de: ("{n} Sitzung", "{n} Sitzungen"),
        it: ("{n} sessione", "{n} sessioni"),
        ja: "{n}件のセッション",
        zh: "{n} 个会话",
        ko: "세션 {n}개",
        ru: ("{n} сессия", "{n} сессии", "{n} сессий"),
    }
}
