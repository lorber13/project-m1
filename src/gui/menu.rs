use eframe::egui::{Ui, Context, CentralPanel, CollapsingHeader};
use crate::{itc::{ScreenshotDim, SettingsEvent}, screens_manager::ScreensManager};
use super::{main_window::CaptureMode, save_settings::SaveSettings};
use std::sync::Arc;
use super::hotkeys_settings::HotkeysSettings;


pub enum MainMenuEvent
{
    ScreenshotRequest(ScreenshotDim, f64),
    SaveConfiguration(SaveSettings),
    HotkeysConfiguration(HotkeysSettings)
}
pub enum MainMenu 
{
    MainWindow(CaptureMode),
    SaveSettings(SaveSettings),
    HotkeysSettings(HotkeysSettings)
}

impl MainMenu
{

    pub fn new() -> Self
    {
        Self::MainWindow(CaptureMode::new())
    }

    pub fn update(&mut self, screens_mgr: Arc<ScreensManager>, save_settings: &SaveSettings, hotkeys_settings: &HotkeysSettings, ctx: &Context, frame: &mut eframe::Frame) -> Option<MainMenuEvent>
    {
        let mut ret = None;
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
                                ui.close_menu();
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
                                    self.switch_to_hotkeys_settings(hotkeys_settings);
                                    click = true;
                                }
                            });
                        });
                    }).response.on_hover_text("Main Menu");
                    //if click {ch.open(Some(false));}
                    
        
                    match *self
                    {
                        Self::MainWindow(_) => ret = self.show_main_window(screens_mgr, ui, ctx, frame),
                        Self::SaveSettings(_) => ret = self.show_save_settings( ui, ctx, frame),
                        Self::HotkeysSettings(_) => ret = self.show_hotkeys_settings( ui, ctx, frame)
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

    fn show_main_window(&mut self, screens_mgr: Arc<ScreensManager>, ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> Option<MainMenuEvent>
    {
        let mut ret = None;
        if let Self::MainWindow(ref mut mw) = self
        {
            //controllo l'utput della main window: se è diverso da None, significa che è stata creata una nuova richiesta di screenshot
            if let Some((area, delay)) = mw.update(ui, screens_mgr, ctx, frame) {
                ret= Some(MainMenuEvent::ScreenshotRequest(area, delay));
            }
        }else {unreachable!();}
        ret
    }

    //-----------------------------SAVE SETTINGS-------------------------------------------------------------------
    fn switch_to_save_settings(&mut self, save_settings: &SaveSettings) 
    {
        if crate::DEBUG {print!("DEBUG: switch to save settings");}
        *self = Self::SaveSettings(save_settings.clone()); //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
    }

    fn show_save_settings(&mut self, ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> Option<MainMenuEvent>
    {
        let mut ret = None;
        if let Self::SaveSettings(ss) = self
        {
            match ss.update(ui, ctx, frame)
            {
                SettingsEvent::Saved => { ret = Some(MainMenuEvent::SaveConfiguration(ss.clone())); self.switch_to_main_window(frame); },
                SettingsEvent::Aborted => self.switch_to_main_window(frame),
                SettingsEvent::Nil => ()
            }  
        }else 
        {unreachable!();}
        ret
    }

     //-----------------------------HOTKEYS SETTINGS-------------------------------------------------------------------
     fn switch_to_hotkeys_settings(&mut self, hotkeys_settings: &HotkeysSettings) 
     {
         if crate::DEBUG {print!("DEBUG: switch to hotkeys settings");}
         *self = Self::HotkeysSettings(hotkeys_settings.clone()); //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
     }
 
     fn show_hotkeys_settings(&mut self, ui: &mut Ui, ctx: &Context, frame: &mut eframe::Frame) -> Option<MainMenuEvent>
     {
         let mut ret = None;
         if let Self::HotkeysSettings(hs) = self
         {
             match hs.update(ui)
             {
                 SettingsEvent::Saved => { ret = Some(MainMenuEvent::HotkeysConfiguration(hs.clone())); self.switch_to_main_window(frame); },
                 SettingsEvent::Aborted => self.switch_to_main_window(frame),
                 SettingsEvent::Nil => ()
             }  
         }else 
         {unreachable!();}
         ret
     }

}

