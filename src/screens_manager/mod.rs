/* Modulo per la gestione di tutti gli schermi a disposizione, inclusa la possibilità di eseguire screenshots.
Mantiene memorizzata una lista contenente, per ogni schermo disponibile, le informazioni principali (id, risoluzione) e uno screenshot fullscreen, utilizzato come icona per rendere riconoscibile lo schermo all'utente.
L'aggiornamento della lista avviene su richiesta, quando viene richiamato <i>update_available_screens()</i>.

Per praticità, il modulo mette a disposizione la possibilità di memorizzare qual'è lo schermo selezionato dall'utente, su cui saranno eseguite le richieste di screenshot.
*/

use image::{imageops::FilterType, RgbaImage};
use screenshots::{DisplayInfo, Screen};
use std::io::Write;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};

pub struct ScreensManager {
    ///Lista di schermi disponibili e relative icone.
    ///Incapsulata in un RwLock per poter:
    ///- essere modificata in mutua esclusione dai metodi interni, pensati essere eseguiti in thread paralleli
    ///a quello principale;
    ///- consentire l'esecuzione del metodo <i>self::select_screen()</i> (che acquisisce il lock in read per ottenere
    ///la lunghezza della lista) mentre il thread della gui accede alla lista per poterla mostrare, permettendo anche ad un
    ///ulteriore thread di accedere alla lista per poter eseguire uno screenshot (vedi <i>self:: start_thread_fullscreen_screenshot()</i>).
    ///
    ///Le immagini associate agli oggetti Screen sono intese come icone, utili per il riconoscimento dello schermo da parte
    ///dell'utente. Sono incapsulate in Mutex per permettere la parallelizzazione dell'operazione di creazione delle icone di
    ///tutti gli schermi collegati (utile perché, in quanto operazioni con le immagini,si tratta di computazione onerosa, ma il modulo è disegnato per
    ///essere scalabile nel numero di schermi).
    screens: RwLock<Vec<(Screen, Mutex<Option<RgbaImage>>)>>, //TO DO: valutare RwLock (al posto del Mutex) anche per le icone
    ///Indice che fa riferimento al vettore <i>self::screens</i>
    curr_screen_index: RwLock<usize>,
    ///Larghezza delle icone che verranno prodotte da <i>self::load_icons()</i>.
    icon_width: u32,
}

impl ScreensManager {
    ///Rileva tutti gli schermi attualmente disponibili e imposta lo schermo primario come quello selezionato
    ///di default.
    pub fn new(icon_width: u32) -> Arc<Self> {
        let ret = Arc::new(Self {
            screens: RwLock::new(vec![]),
            curr_screen_index: RwLock::new(0),
            icon_width,
        });
        ret.update_available_screens();
        ret.select_primary_screen();
        ret
    }

    ///Aggiorna il vettore di Screen, rilevando le modifiche hardware.
    /// Anche l'indice viene modificato, nel caso lo schermo precedentemente selezionato cambi
    /// di posizione nel vettore.
    /// Nel caso lo schermo precedentemente selezionato non venga piu' rilevato,
    /// di default viene selezionato quello primario.
    ///
    ///Tutte le operazioni sono eseguite in modo asincrono rispetto al thread che ha richiamato il metodo.
    ///Infatti, essendo il modulo pensato per essere scalabile nel numero di schermi, la lista può diventare lunga
    ///e le operazioni su di essa onerose.
    ///Per poter eseguire l'elaborazione, il thread dovrà ottenere il lock di <i>self::screens</i> in modalità write.
    ///Un altro thread che volesse quindi essere
    pub fn update_available_screens(self: &Arc<Self>) {
        let arc_clone = self.clone();
        std::thread::spawn(move || {
            let curr_id = if !arc_clone.get_screens().is_empty() {
                Some(arc_clone.get_current_screen_infos().unwrap().id)
            } else {
                None
            };

            {
                let mut write_lk = arc_clone.screens.write().unwrap();
                write_lk.clear();
                for s in Screen::all().unwrap() {
                    write_lk.push((s, Mutex::new(None)));
                }
            }
            arc_clone.load_icons();

            if let Some(id) = curr_id {
                match arc_clone
                    .get_screens()
                    .iter()
                    .position(|s| s.0.display_info.id == id)
                {
                    Some(i) => *arc_clone.curr_screen_index.write().unwrap() = i,
                    None => arc_clone.select_primary_screen(),
                }
            }
        });
    }

    ///Tra gli schermi disponibili (ottenuti dall'ultima rilevazione), permette di selezionare quello su cui
    ///verranno eseguiti i prossimi screenshots.
    ///Controlla che l'indice passato sia compatibile con la lunghezza della lista degli schermi. Deve quindi
    ///leggere <i>self::screens</i>, dopo averne ottenuto il lock in lettura. Questo implica che non si possa cambiare la
    ///selezione durante l'esecuzione di un refresh, ma che si possa fare mentre la lista <i>self::screens</i> viene acceduta
    ///per essere mostrata.
    pub fn select_screen(self: &Arc<Self>, index: usize) {
        if index < self.get_screens().len() {
            *self.curr_screen_index.write().unwrap() = index;
        }
    }

    ///Richiama il metodo <i>self::select_screen()</i> passando come indice quello dello schermo che rileva
    ///come primario.
    ///Per trovare tale indice, è necessario ottenere il lock in lettura su <i>self::screens</i>: questo è implicitamente fatto con
    ///la chiamata a <i>self::get_screens()</i>.
    pub fn select_primary_screen(self: &Arc<Self>) {
        if let Some(i) = self
            .get_screens()
            .iter()
            .position(|s| s.0.display_info.is_primary)
        {
            *self.curr_screen_index.write().unwrap() = i;
        }
    }

    ///Lancia un thread che:
    ///- esegue uno screenshot sullo schermo attualmente selezionato;
    ///- invia l'immagine sul canale il cui <i>Receiver</i> è ritornato dal metodo corrente.
    ///Oppure invia sul canale un messaggio di errore.
    pub fn start_thread_fullscreen_screenshot(
        self: &Arc<Self>,
    ) -> Receiver<Result<RgbaImage, &'static str>> {
        let (tx, rx) = channel();
        let sc = self.clone();
        std::thread::spawn(move || {
            tx.send(sc.fullscreen_screenshot()).expect(
                "thread performing fullscreen screenshot was not able to send through the channel",
            );
        });
        rx
    }

    ///Ottiene lock in lettura su <i>self::screens</i> per poter accedere alla struttura Screen relativa
    ///allo schermo attualmente selezionato e richiamare <i>capture()</i> su essa.
    ///L'acquisizione del lock implica che il metodo corrente si blocchi se è contemporaneamente eseguito l'aggiornamento di tale lista.
    fn fullscreen_screenshot(self: &Arc<Self>) -> Result<RgbaImage, &'static str> {
        match self
            .get_screens()
            .get(*self.curr_screen_index.read().unwrap())
            .unwrap()
            .0
            .capture()
        {
            Ok(shot) => Ok(shot),
            Err(s) => {
                let _ = write!(
                    std::io::stderr(),
                    "Error: unable to perform screenshot: {:?}",
                    s
                );
                Err("Error: unable to perform screenshot")
            }
        }
    }

    pub fn get_current_screen_index(self: &Arc<Self>) -> usize {
        *self.curr_screen_index.read().unwrap()
    }

    ///Ritorna None nel caso le info sugli schermi non siano ancora state caricate (vettore di screen vuoto).
    pub fn get_current_screen_infos(self: &Arc<Self>) -> Option<DisplayInfo> {
        self.get_screens()
            .get(*self.curr_screen_index.read().unwrap())
            .map(|(screen, _)| screen.display_info)
    }

    /// Lancia un thread per ogni screen nel vettore di screen per parallelizzare la creazione di tutte le corrispondenti icone.
    /// In particolare, ogni thread scatta uno screenshot del proprio schermo, poi ridimensiona l'immagine
    /// (riducendola alla dimensione specificata in ScreensManager::icon_width) e la salva nella corretta posizione all'interno
    /// del vettore di screen.
    fn load_icons(self: &Arc<Self>) {
        for (index, _) in self.get_screens().iter().enumerate() {
            let arc = self.clone();
            std::thread::spawn(move || {
                let screens = arc.get_screens();
                let (s, i) = screens.get(index).unwrap();
                let img = s.capture().unwrap();
                let height = arc.icon_width * img.height() / img.width();
                let icon = image::imageops::resize(
                    &s.capture().unwrap(),
                    arc.icon_width,
                    height,
                    FilterType::Gaussian,
                );
                let mut g = i.lock().unwrap();
                *g = Some(icon);
            });
        }
    }

    pub fn try_get_screens<'a>(
        self: &'a Arc<Self>,
    ) -> Option<RwLockReadGuard<'a, Vec<(Screen, Mutex<Option<RgbaImage>>)>>> {
        match self.screens.try_read() {
            Ok(g) => Some(g),
            Err(..) => None,
        }
    }

    fn get_screens<'a>(
        self: &'a Arc<Self>,
    ) -> RwLockReadGuard<'a, Vec<(Screen, Mutex<Option<RgbaImage>>)>> {
        self.screens.read().unwrap()
    }
}
