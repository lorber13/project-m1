use eframe::egui::{Ui, Context, CentralPanel};
use crate::{itc::{ScreenshotDim, SettingsEvent}, screens_manager::ScreensManager, hotkeys::RegisteredHotkeys};
use super::{main_window::CaptureMode, save_settings::SaveSettings};
use std::sync::{Arc, mpsc::TryRecvError};
use super::hotkeys_settings::HotkeysSettings;
use std::sync::mpsc::Receiver;


pub enum MainMenuEvent
{
    ScreenshotRequest(ScreenshotDim, f64),
    SaveConfiguration(SaveSettings),
    HotkeysConfiguration(Arc<RegisteredHotkeys>),
    Error(&'static str),
    Nil
}
pub enum MainMenu 
{
    MainWindow(CaptureMode),
    SaveSettings(SaveSettings),
    LoadingHotkeysSettings(Receiver<Arc<RegisteredHotkeys>>),
    HotkeysSettings(HotkeysSettings, Arc<RegisteredHotkeys>)
}

impl MainMenu
{

    pub fn new() -> Self
    {
        Self::MainWindow(CaptureMode::new())
    }

    pub fn update(&mut self, screens_mgr: Arc<ScreensManager>, save_settings: &SaveSettings, registered_hotkeys: Arc<RegisteredHotkeys>, ctx: &Context, frame: &mut eframe::Frame) -> MainMenuEvent
    {
        let mut ret = MainMenuEvent::Nil;
        CentralPanel::default().show(ctx, |ui|
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
                                    self.switch_to_save_settings(save_settings);
                                    click = true;
                                }

                                if ui.button("Hotkeys Settings").clicked()
                                {
                                    ui.close_menu();
                                    self.switch_to_hotkeys_settings(registered_hotkeys);
                                    click = true;
                                }
                            });
                        });
                    });
                    //if click {ch.open(Some(false));}
                    
        
                    match *self
                    {
                        Self::MainWindow(_) => ret = self.show_main_window(screens_mgr, ui, ctx, frame),
                        Self::SaveSettings(_) => ret = self.show_save_settings( ui, ctx, frame),
                        Self::HotkeysSettings(..) => ret = self.show_hotkeys_settings( ui, frame),
                        Self::LoadingHotkeysSettings(..) => ret = self.load_hotkeys_settings()
                    }
        
                });
        });
        
        ret
    } 

    
    /*----------------MAIN WINDOW------------------------------------------ */

    fn switch_to_main_window(&mut self,  _frame: &mut eframe::Frame)
    {
        *self = Self::MainWindow(CaptureMode::new());
    }

    fn show_main_window(&mut self, screens_mgr: Arc<ScreensManager>, ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> MainMenuEvent
    {
        let mut ret = MainMenuEvent::Nil;
        if let Self::MainWindow(ref mut mw) = self
        {
            //controllo l'utput della main window: se è diverso da None, significa che è stata creata una nuova richiesta di screenshot
            if let Some((area, delay)) = mw.update(ui, screens_mgr, ctx, frame) {
                ret= MainMenuEvent::ScreenshotRequest(area, delay);
            }
        }else {unreachable!();}
        ret
    }

    //-----------------------------SAVE SETTINGS-------------------------------------------------------------------
    fn switch_to_save_settings(&mut self, save_settings: &SaveSettings) 
    {
        if crate::DEBUG {print!("DEBUG: switch to save settings");}
        match self
        {
            Self::SaveSettings(_) => (), //non c'è nulla di nuovo da visualizzare
            _ => *self = Self::SaveSettings(save_settings.clone()) //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
        }
        
    }

    fn show_save_settings(&mut self, ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> MainMenuEvent
    {
        let mut ret = MainMenuEvent::Nil;
        if let Self::SaveSettings(ss) = self
        {
            match ss.update(ui, ctx)
            {
                SettingsEvent::Saved => { ret = MainMenuEvent::SaveConfiguration(ss.clone()); self.switch_to_main_window(frame); },
                SettingsEvent::Aborted => self.switch_to_main_window(frame),
                SettingsEvent::Error(e) => ret = MainMenuEvent::Error(e),
                SettingsEvent::Nil => ()
            }  
        }else 
        {unreachable!();}
        ret
    }

     //-----------------------------HOTKEYS SETTINGS-------------------------------------------------------------------
     fn switch_to_hotkeys_settings(&mut self, registered_hotkeys: Arc<RegisteredHotkeys>) 
     {
         if crate::DEBUG {print!("DEBUG: switch to hotkeys settings");}
         match self {
             Self::HotkeysSettings(..) | Self::LoadingHotkeysSettings(..) => (), //non c'è nulla di nuovo da visualizzare
             _ => *self = Self::LoadingHotkeysSettings(registered_hotkeys.create_copy()) //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
         }
         
     }

     fn load_hotkeys_settings(&mut self) -> MainMenuEvent
     {
        let mut ret = MainMenuEvent::Nil;
        if let Self::LoadingHotkeysSettings(r) = self
        {
            match r.try_recv()
            {
                Ok(rh) => *self = Self::HotkeysSettings(HotkeysSettings::new(), rh), //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
                Err(TryRecvError::Disconnected) => ret= MainMenuEvent::Error("Loading failed"),
                Err(TryRecvError::Empty) => ()
            }
        }else {unreachable!();}
        ret
     }
 
     fn show_hotkeys_settings(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) -> MainMenuEvent
     {
         let mut ret = MainMenuEvent::Nil;
         if let Self::HotkeysSettings(hs, rh) = self
         {
             match hs.update(ui, rh.clone())
             {
                 SettingsEvent::Saved => { ret = MainMenuEvent::HotkeysConfiguration(rh.clone()); self.switch_to_main_window(frame); },
                 SettingsEvent::Aborted => self.switch_to_main_window(frame),
                 SettingsEvent::Error(e) => ret = MainMenuEvent::Error(e),
                 SettingsEvent::Nil => ()
             }  
         }else 
         {unreachable!();}
         ret
     }

}

