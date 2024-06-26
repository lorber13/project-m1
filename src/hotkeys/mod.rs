use eframe::egui::Context;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, RwLock};

///Il numero di varianti della enum HotkeyName. Il modulo Hotkeys è predisposto per scalare ad un maggiore
///numero di hotkeys.
pub const N_HOTK: usize = 2;

/// Può esserci una sola combinazione di tasti associata ad ogni variante di questa enum. Infatti, ad ogni variante di HotkeyName è associato un comando che può essere dato in input al programma.
///
/// <b>Attenzione:</b> se si dovessero aggiungere varianti a questa enum, è necessario aggiornare la costante <i>N_HOTK</i>.
#[derive(Clone, Copy)]
pub enum HotkeyName {
    FullscreenScreenshot,
    RectScreenshot,
}

impl PartialEq for HotkeyName {
    fn eq(&self, other: &Self) -> bool {
        <HotkeyName as Into<usize>>::into(*self) == <HotkeyName as Into<usize>>::into(*other)
    }
}

impl Into<usize> for HotkeyName {
    /// Converte ad intero assegnando un indice incrementale ad ogni variante della enum HotkeyName.
    fn into(self) -> usize {
        match self {
            Self::FullscreenScreenshot => 0,
            Self::RectScreenshot => 1,
        }
    }
}

impl Into<String> for HotkeyName {
    fn into(self) -> String {
        match self {
            Self::FullscreenScreenshot => String::from("Fullscreen screenshot"),
            Self::RectScreenshot => String::from("Rect screenshot"),
        }
    }
}

impl From<usize> for HotkeyName {
    /// A partire da un intero, lo converte nella variante che compare in quella posizione nella definizione della enum.
    fn from(us: usize) -> Self {
        match us {
            0 => Self::FullscreenScreenshot,
            1 => Self::RectScreenshot,
            _ => unreachable!("Invalid value in HotkeyName::from::<usize>()"),
        }
    }
}

/// Struttura dati che si occupa di gestire le hotkeys registrate al livello dell'intera applicazione.<br>
/// Memorizza al suo interno:
/// - copia di backup: campo privato, modificabile solo con la chiamata al metodo <i>update_changes()</i>;
/// - <i>vec</i>: copia "di brutta" del precedente campo. Usata per salvare le modifiche temporanee prima del loro effettivo salvataggio.<br>
///
/// Questa ridondanza ha l'obiettivo di mantenere stabili le impostazioni originali fino a quando le modifiche non
/// vengono confermate. Infatti, solo <i>vec</i> viene modificato tramite le chiamate a <i>request_register()</i> e
/// <i>request_unregister()</i>.<br>
/// Prima dell'utilizzo di <i>vec</i> per introdurre nuove modifiche deve essere eseguito il metodo <i>prepare_for_updates()</i>,
/// che copia il contenuto di <i>backup</i> in <i>vec</i>.<br>
///
/// Esiste la possibilità di disabilitare l'ascolto delle hotkeys, tramite il campo <i>listen_enabled</i> e il relativo metodo
/// setter.
///
/// I campi che possono essere modificati sono protetti da RwLock per soddisfare i seguenti requisiti:
/// - mutabilità interna: il campo deve poter essere modificato, permettendo all'intera struttura di essere posseduta da Arc;
/// - la struttura deve essere thread-safe, quindi il tipo Cell non sarebbe bastato.<br>
///
/// Si è deciso di incapsulare ogni cella dei vettori (<i>backup, vec</i>) in un RwLock e non incapsulare ciascun vettore in un unico
/// RwLock per permettere maggiore parallelismo nel loro accesso.
pub struct RegisteredHotkeys {
    ///Memorizzazione stabile delle hotkey registrate. Questo Vec viene modificato solo quando una modifica viene salvata.
    ///Si fa riferimento al contenuto di questo <i>Vec</i> per sapere quali comandi devono essere eseguiti in seguito alla
    ///pressione delle hotkeys durante il funzionamento normale del programma.
    backup: Vec<RwLock<Option<(HotKey, String)>>>,
    ///Copia di "brutta" del vettore di Hotkeys, modificato direttamente durante il settaggio delle impostazioni.
    vec: Vec<RwLock<Option<String>>>,
    ///Mette a disposizione i metodi per attivare/disattivare l'effettivo ascolto delle hotkeys.
    ghm: GlobalHotKeyManager,
    ///Per disattivare temporaneamente le Hotkeys senza dover richiamare <i>unregister()</i>.
    listen_enabled: RwLock<bool>,
}

impl RegisteredHotkeys {
    const CONFIG_FILE_NAME: &'static str = ".config_hotkeys";

    /// Controlla se esiste il file in cui sono state salvate permanentemente le impostazioni: se esiste,
    /// lo legge e usa le informazioni per riempire la nuova struct.
    /// Altrimenti, assegna alla nuova struct valori di default:
    /// Crea i due <i>Vec</i> di <i>RwLock</i> inizialmente vuoti.
    ///Imposta <i>listen_enabled</i> a true di default.
    ///
    ///Ritorna la struttura già incapsulata in un <i>Arc</i>.
    pub fn new() -> Arc<Self> {
        let mut vec = vec![];
        let mut backup = vec![];
        for _ in 0..N_HOTK {
            vec.push(RwLock::new(None));
            backup.push(RwLock::new(None));
        }
        let ret = Arc::new(Self {
            vec,
            backup,
            ghm: GlobalHotKeyManager::new().unwrap(),
            listen_enabled: RwLock::new(true),
        });

        if let Ok(f) = File::open(Self::CONFIG_FILE_NAME) {
            ret.deserialize(f);
        }
        ret
    }

    pub fn deserialize(self: &Arc<Self>, f: File) {
        let mut buf = BufReader::new(f);
        let mut line = String::new();
        let mut i = 0;
        while let Ok(n) = buf.read_line(&mut line) {
            if n == 0 {
                break;
            }
            let _ = line.pop();
            if n > 1 {
                self.vec
                    .get(i)
                    .unwrap()
                    .write()
                    .unwrap()
                    .replace(line.clone());
            }
            i += 1;
            line = String::new();
        }
        let _ = self.update_changes();
    }

    pub fn start_thread_serialize(self: &Arc<Self>) {
        let arc_clone = self.clone();
        std::thread::spawn(move || {
            if let Ok(mut f) = File::create(Self::CONFIG_FILE_NAME) {
                for rl in arc_clone.backup.iter() {
                    let g = rl.read().unwrap().clone();
                    if let Some((_, s)) = g {
                        let mut s_clone = s.clone();
                        s_clone.push('\n');
                        let _ = f.write(s_clone.as_bytes());
                    } else {
                        let s = String::from("\n");
                        let _ = f.write(s.as_bytes());
                    }
                }
            }
        });
    }

    ///Copia il contenuto di <i>self::vec</i> dentro a <i>self::backup</i>,
    ///andando a richiamare <i>self:: register()/unregister()<i> in base alle differenze tra le celle
    ///dei due vettori associate alla stessa <i>HotkeyName</i>.
    ///In particolare, se per una determinata <i>HotkeyName</i> era già stata memorizzata una combinazione di
    ///tasti (CT), si ha l'accortezza di richiamare i metodi per la registrazione solo se effettivamente la CT è
    ///cambiata: questo può essere rilevato convertendo le CT in stringhe ed eseguendo il metodo <i>cmp()</i>.
    ///
    ///Ad ogni operazione di registrazione, controlla se si sono verificati errori.
    pub fn update_changes(self: &Arc<Self>) -> Result<(), String> {
        let mut ret = Ok(());
        for i in 0..N_HOTK {
            let temp1;
            let temp2;
            {
                temp1 = self.vec.get(i).unwrap().read().unwrap().clone();
                temp2 = self.backup.get(i).unwrap().read().unwrap().clone();
            }
            match (temp1, temp2) {
                (None, None) => (),
                (None, Some(..)) => ret = self.unregister(HotkeyName::from(i)),
                (Some(s), None) => ret = self.register(s.to_string(), HotkeyName::from(i)),
                (Some(s1), Some((_, s2))) => {
                    if s1.cmp(&s2) != Ordering::Equal {
                        ret = self.register(s1.to_string(), HotkeyName::from(i))
                    }
                }
            }

            ret.as_ref()?; //ritorna nel caso ret.is_err() == true
        }

        ret
    }

    ///Metodo da richiamare <b>sempre</b> prima di iniziare una sessione di modifica.
    ///Copia il contenuto di <i>self::backup</i> in <i>self::vec</i> in modo che quest'ultimo possa essere
    ///modificato a partire da dati consistenti.
    /// Per non bloccare il main thread e rendere l'operazione veloce a prescindere dal numero di hotkeys,
    /// un thread padre lancia un figlio per ogni entry dei vettori, i thread figli eseguono la copia in
    /// parallelo.
    ///
    ///<b>Ritorna:</b> un <i>Receiver</i> su cui è possibile mettersi in ascolto per attendere che l'operazione di copia
    ///termini.
    pub fn prepare_for_updates(self: &Arc<Self>) -> Receiver<()> {
        let (tx, rx) = channel();
        let self_clone = self.clone();

        std::thread::spawn(move || {
            let mut jh = vec![];
            for i in 0..N_HOTK {
                let self_clonex2 = self_clone.clone();
                jh.push(std::thread::spawn(move || {
                    let temp1;
                    let temp2;
                    {
                        temp1 = self_clonex2.vec.get(i).unwrap().read().unwrap().clone();
                        temp2 = self_clonex2.backup.get(i).unwrap().read().unwrap().clone();
                    }
                    match (temp1, temp2) {
                        (None, None) => (),
                        (None, Some((_, s))) => {
                            self_clonex2
                                .vec
                                .get(i)
                                .unwrap()
                                .write()
                                .unwrap()
                                .replace(s.clone());
                        }
                        (Some(_), None) => {
                            self_clonex2.vec.get(i).unwrap().write().unwrap().take();
                        }
                        (Some(s1), Some((_, s2))) => {
                            if s1.cmp(&s2) != Ordering::Equal {
                                self_clonex2
                                    .vec
                                    .get(i)
                                    .unwrap()
                                    .write()
                                    .unwrap()
                                    .replace(s2.clone());
                            }
                        }
                    }
                }))
            }
            for j in jh {
                let _ = j.join();
            }
            let _ = tx.send(());
        });
        rx
    }

    ///Esegue un ciclo su tutte le hotkeys memorizzate nella bozza (<i>self::vec</i>)
    /// e le confronta con quella passata come parametro.
    fn check_if_already_registered(self: &Arc<Self>, hotkey: &String) -> bool {
        for opt in self.vec.iter() {
            if let Some(s) = &*opt.read().unwrap() {
                if s == hotkey {
                    return true;
                }
            }
        }

        false
    }

    ///Memorizza l'associazione tra la hotkey <i>name</i> e la combinazione di tasti scritta sotto forma di stringa <i>h_str</i>.
    ///Per controllare la correttezza sintattica della stringa utilizza <i>Hotkey::from_str().is_ok()</i>.
    ///
    ///<b>ATTENZIONE:</b> con questo metodo, si sta solo creando una <b>richiesta</b> di registrazione, che si tradurrà nella registrazione della hotkey
    ///solo quando verrà richiamato <i>self::update_changes()</i>.
    ///
    /// Le operazioni sono eseguite in un thread separato, il quale al termine invierà un segnale tramite il <i>tx</i> passato.
    ///Si è deciso di parallelizzare visto il design scalabile del modulo: è previsto che la lista di Hotkeys possa diventare
    ///più lunga, quindi la sua lettura integrale più onerosa.
    pub fn request_register(
        self: &Arc<Self>,
        h_str: String,
        name: HotkeyName,
        tx: Sender<Result<(), &'static str>>,
    ) {
        let self_clone = self.clone();

        std::thread::spawn(move || {
            let mut ret = Ok(());

            //controllo che la stessa combinazione di tasti non sia già associata ad un altro comando:
            if self_clone.check_if_already_registered(&h_str) {
                ret = Err("Hotkey already registered");
            } else if HotKey::from_str(&h_str).is_ok() {
                self_clone
                    .vec
                    .get(<HotkeyName as Into<usize>>::into(name))
                    .unwrap()
                    .write()
                    .unwrap()
                    .replace(h_str);
            }

            let _ = tx.send(ret);
        });
    }

    ///Esegue la registrazione della hotkey presso il <i>GlobalHotkeyManager</i>.
    ///Se la registrazione ha avuto successo, aggiorna in <i>self::backup<i> l'informazione relativa alla
    /// <i>HotkeyName</i> passata come parametro. Altrimenti, ritorna una stringa di errore. <br/>
    /// NON è possibile fare eseguire da un thread separato perché non compatibile con i requisiti del crate GlobalHotkey.
    fn register(self: &Arc<Self>, h_str: String, name: HotkeyName) -> Result<(), String> {
        if let Ok(h) = HotKey::from_str(&h_str) {
            return match self.ghm.register(h) {
                Ok(()) => {
                    self.backup
                        .get(<HotkeyName as Into<usize>>::into(name))
                        .unwrap()
                        .write()
                        .unwrap()
                        .replace((h, h_str));
                    Ok(())
                }
                Err(e) => Err(format!(
                    "Unable to register the hotkey related to command {}.\nError: {}",
                    <HotkeyName as Into<String>>::into(name),
                    e
                )),
            };
        }

        Err(format!(
            "Unable to register the hotkey related to command {}",
            <HotkeyName as Into<String>>::into(name)
        ))
    }

    ///Cancella l'associazione tra la hotkey <i>name</i> e la combinazione di tasti memorizzata nella corrispondente entry di <i>self::vec</i>.
    ///
    ///<b>ATTENZIONE:</b> con questo metodo, si sta solo modificando la copia temporanea <i>self::vec</i>.
    ///Le modifiche possono essere rese definitive richiamando <i>self::update_changes()</i>.

    pub fn request_unregister(self: &Arc<Self>, name: HotkeyName) {
        let _ = self
            .vec
            .get(<HotkeyName as Into<usize>>::into(name))
            .unwrap()
            .write()
            .unwrap()
            .take();
    }

    ///Legge, da <i>self::backup</i>, qual'è la combinazione di tasti associata alla hotkey <i>name</i>.
    ///Se trova una combinazione valida, chiede l'annullamento della registrazione presso il <i>GlobalHotkeyManager</i>
    ///e ritorna l'esito di tale operazione.
    ///
    /// NON è possibile fare eseguire da un thread diverso dal main thread a causa dei requisiti del crate GlobalHotkeys.
    fn unregister(self: &Arc<Self>, name: HotkeyName) -> Result<(), String> {
        let temp = self
            .backup
            .get(<HotkeyName as Into<usize>>::into(name))
            .unwrap()
            .write()
            .unwrap()
            .take();
        if let Some((h, _)) = temp {
            if self.ghm.unregister(h).is_ok() {
                return Ok(());
            }
        }
        Err(format!(
            "Unable to unregister the hotkey related to command {}",
            <HotkeyName as Into<String>>::into(name)
        ))
    }

    ///Ritorna la combinazione di tasti associata alla hotkey <i>name</i> espressa come stringa di tasti separati dal carattere '+'.
    ///Siccome il metodo è pensato per poter essere usato durante la modifica delle <i>RegisteredHotkeys</i> da parte
    ///di una schermata di impostazioni, quello che è ritornato è il contenuto di <i>self::vec</i> e non di <i>self::backup</i>.
    pub fn get_hotkey_string(self: &Arc<Self>, name: HotkeyName) -> Option<String> {
        self.vec
            .get(<HotkeyName as Into<usize>>::into(name))
            .unwrap()
            .read()
            .unwrap()
            .as_deref()
            .map(|hk_str| hk_str.to_string())
    }

    pub fn set_listen_enabled(&self, val: bool) {
        *self.listen_enabled.write().unwrap() = val;
    }
}

/// Funzione che lancia un thread worker che rimane (con chiamata bloccante recv()) in ascolto di eventi di pressione di
/// hotkeys. Riceve come parametro il <i>Context</i> della gui per poter svegliare la gui, in qualsiasi stato essa sia,
/// dopo il verificarsi di un evento. In particolare, questo è utile nel momento in cui l'applicazione ha smesso
/// di eseguire il metodo <i>App::update()</i> (vedi impl <i>GlobalGuiState</i>) perché la finestra non è al momento visibile.
///
/// Quando la chiamata a <i>GlobalHotkeyEvent::receiver.recv()</i> ritorna un evento <i>GlobalHotkeyEvent<i>, esso viene
/// convertito in <i>HotkeyName<i> utilizzando la struttura <i>RegisteredHotkeys</i> e inviato sul canale con il thread gui.
/// Successivamente, si assicura che la gui possa leggere dal canale, svegliandola con il metodo <i>Context::request_repaint()</i>.
pub fn start_thread_listen_hotkeys(
    arc_ctx: Arc<Context>,
    arc_registered_hotkeys: Arc<RegisteredHotkeys>,
    main_thr_channel: Sender<HotkeyName>,
) {
    std::thread::spawn(move || loop {
        if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
            for (i, opt) in arc_registered_hotkeys.backup.iter().enumerate() {
                match opt.read().unwrap().clone() {
                    None => (),
                    Some((h, _)) => {
                        if h.id() == event.id {
                            main_thr_channel.send(HotkeyName::from(i)).unwrap();
                            arc_ctx.request_repaint();
                        }
                    }
                }
            }
        }
    });
}
