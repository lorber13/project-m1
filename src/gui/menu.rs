use eframe::egui::{Ui, Context, CentralPanel};
use crate::{itc::{ScreenshotDim, SettingsEvent}, screens_manager::ScreensManager, hotkeys::RegisteredHotkeys};
use super::{capture_mode::CaptureMode, save_settings::SaveSettings};
use std::{sync::{Arc, mpsc::TryRecvError}, cell::RefCell};
use super::hotkeys_settings::HotkeysSettings;
use std::sync::mpsc::Receiver;
use std::rc::Rc;

pub enum MainMenuEvent
{
    ScreenshotRequest(ScreenshotDim, f64),
    SaveConfiguration(SaveSettings),
    HotkeysConfiguration(Arc<RegisteredHotkeys>),
    Nil
}
enum MainMenuState 
{
    CaptureMode(CaptureMode),
    SaveSettings(SaveSettings),
    LoadingHotkeysSettings(Receiver<Arc<RegisteredHotkeys>>),
    HotkeysSettings(HotkeysSettings, Arc<RegisteredHotkeys>)
}

pub struct MainMenu
{
    state : MainMenuState,
    alert: Rc<RefCell<Option<&'static str>>>, 
    screens_mgr: Arc<ScreensManager>, 
    save_settings: Rc<SaveSettings>,
    registered_hotkeys: Arc<RegisteredHotkeys>,
}

impl MainMenu
{

    pub fn new(alert: Rc<RefCell<Option<&'static str>>>, screens_mgr: Arc<ScreensManager>, save_settings: Rc<SaveSettings>, registered_hotkeys: Arc<RegisteredHotkeys>) -> Self
    {
        Self {state: MainMenuState::CaptureMode(CaptureMode::new(screens_mgr.clone())), screens_mgr, alert, save_settings, registered_hotkeys}
    }

    pub fn update(&mut self, enabled: bool, ctx: &Context, frame: &mut eframe::Frame) -> MainMenuEvent
    {
        let mut ret = MainMenuEvent::Nil;
        CentralPanel::default().show(ctx, |ui|
        {
            ui.add_enabled_ui(enabled, |ui|
            {
                ui.horizontal(|ui|
                    {
                            let mut click = false;
                            
                            ui.menu_button("☰", |ui|
                            {
                                ui.vertical(|ui|
                                {
        
                                    if ui.button("Capture").clicked()
                                    {
                                        self.switch_to_main_window(frame);
                                        click = true;
                                    }
                                    ui.menu_button("Settings...", |ui|
                                    {
                                        if ui.button("Save Settings").clicked()
                                        {
                                            ui.close_menu();
                                            self.switch_to_save_settings();
                                            click = true;
                                        }
        
                                        if ui.button("Hotkeys Settings").clicked()
                                        {
                                            ui.close_menu();
                                            self.switch_to_hotkeys_settings();
                                            click = true;
                                        }
                                    });
                                });
                            });
                            //if click {ch.open(Some(false));}
                            
                
                            ui.vertical(|ui|
                            {
                                ui.add_space(5.0);
        
                                match self.state
                                {
                                    MainMenuState::CaptureMode(_) =>{ ret = self.show_main_window( ui, ctx, frame); },
                                    MainMenuState::SaveSettings(_) =>{ ret = self.show_save_settings( ui, ctx, frame); },
                                    MainMenuState::HotkeysSettings(..) =>{ ret = self.show_hotkeys_settings( ui, ctx,frame); },
                                    MainMenuState::LoadingHotkeysSettings(..) =>{ ret = self.load_hotkeys_settings(); }
                                }
                            });
                        });
            });
            
        });
        
        ret
    } 

    
    /*----------------MAIN WINDOW------------------------------------------ */

    fn switch_to_main_window(&mut self,  _frame: &mut eframe::Frame)
    {
        match self.state
        {
            MainMenuState::CaptureMode(..) => (), //non c'è niente di nuovo da visualizzare
            _ => self.state = MainMenuState::CaptureMode(CaptureMode::new(self.screens_mgr.clone()))
        }
        
    }

    fn show_main_window(&mut self, ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> MainMenuEvent
    {
        let mut ret = MainMenuEvent::Nil;
        if let MainMenuState::CaptureMode(ref mut cm) = self.state
        {
            //controllo l'utput della main window: se è diverso da None, significa che è stata creata una nuova richiesta di screenshot
            if let Some((area, delay)) = cm.update(ui, ctx, frame) {
                ret= MainMenuEvent::ScreenshotRequest(area, delay);
            }
        }else {unreachable!();}
        ret
    }

    //-----------------------------SAVE SETTINGS-------------------------------------------------------------------
    fn switch_to_save_settings(&mut self) 
    {
        if crate::DEBUG {print!("DEBUG: switch to save settings");}
        match self.state
        {
            MainMenuState::SaveSettings(_) => (), //non c'è nulla di nuovo da visualizzare
            _ => self.state = MainMenuState::SaveSettings(SaveSettings::clone(&*self.save_settings)) //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
        }
        
    }

    fn show_save_settings(&mut self, ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> MainMenuEvent
    {
        let mut ret = MainMenuEvent::Nil;
        if let MainMenuState::SaveSettings(ss) = &mut self.state
        {
            match ss.update(ui, ctx)
            {
                SettingsEvent::Saved => { ret = MainMenuEvent::SaveConfiguration(ss.clone()); self.switch_to_main_window(frame); },
                SettingsEvent::Aborted => self.switch_to_main_window(frame),
                SettingsEvent::Nil => ()
            }  
        }else 
        {unreachable!();}
        ret
    }

     //-----------------------------HOTKEYS SETTINGS-------------------------------------------------------------------
     fn switch_to_hotkeys_settings(&mut self) 
     {
         if crate::DEBUG {print!("DEBUG: switch to hotkeys settings");}
         match self.state {
             MainMenuState::HotkeysSettings(..) | MainMenuState::LoadingHotkeysSettings(..) => (), //non c'è nulla di nuovo da visualizzare
             _ => self.state = MainMenuState::LoadingHotkeysSettings(self.registered_hotkeys.create_copy()) //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
         }
         
     }

     fn load_hotkeys_settings(&mut self) -> MainMenuEvent
     {
        let ret = MainMenuEvent::Nil;
        if let MainMenuState::LoadingHotkeysSettings(r) = &mut self.state
        {
            match r.try_recv()
            {
                Ok(rh) => self.state = MainMenuState::HotkeysSettings(HotkeysSettings::new(self.alert.clone(), rh.clone()), rh.clone()), //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
                Err(TryRecvError::Disconnected) => {self.alert.borrow_mut().replace("Loading failed");},
                Err(TryRecvError::Empty) => ()
            }
        }else {unreachable!();}
        ret
     }
 
     fn show_hotkeys_settings(&mut self,  ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> MainMenuEvent
     {
         let mut ret = MainMenuEvent::Nil;
         if let MainMenuState::HotkeysSettings(hs, rh) = &mut self.state
         {
            self.registered_hotkeys.set_listen_enabled(false);
             match hs.update(ui, ctx)
             {
                 SettingsEvent::Saved => { ret = MainMenuEvent::HotkeysConfiguration(rh.clone()); self.registered_hotkeys = rh.clone(); self.switch_to_main_window(frame); },
                 SettingsEvent::Aborted => self.switch_to_main_window(frame),
                 SettingsEvent::Nil => ()
             }  
         }else 
         {unreachable!();}
         ret
     }

}

