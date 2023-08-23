//Processo che, appoggiandosi sul crate winit, crea una finestra (trasparente, fullscreen) per rilevare
//la risoluzione (in pixel) dello schermo.
//Permette di ottenere la risoluzione su qualsiasi schermo e qualsiasi piattaforma.

fn main() {
    let el = winit::event_loop::EventLoop::new();
    let w = winit::window::Window::new(&el).unwrap();
    w.set_visible(false);
    w.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
    let sf = w.scale_factor();
    let s = egui_winit::screen_size_in_pixels(&w); 
    println!("width: {} height: {} scale factor: {}", s.x, s.y, sf);
}
