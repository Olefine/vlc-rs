// Copyright (c) 2015 T. Okubo
// This file is part of vlc-rs.
// Licensed under the MIT license, see the LICENSE file.

use std::ptr::null;
use std::borrow::Cow;
use std::marker::PhantomData;
use ffi;
use ::tools::{to_cstr, from_cstr, from_cstr_ref};
use ::libc::{c_void, c_char, c_int};
use ::enums::*;

/// Retrieve libvlc version. 
pub fn version() -> Cow<'static, str> {
    unsafe{
        from_cstr_ref(ffi::libvlc_get_version()).unwrap()
    }
}

/// Retrieve libvlc compiler version.
pub fn compiler() -> Cow<'static, str> {
    unsafe{
        from_cstr_ref(ffi::libvlc_get_compiler()).unwrap()
    }
}

pub struct Instance {
    pub ptr: *mut ffi::libvlc_instance_t,
}

impl Instance {
    /// Create and initialize a libvlc instance. 
    pub fn new() -> Option<Instance> {
        unsafe{
            let p = ffi::libvlc_new(0, null());
            
            if p.is_null() {
                return None;
            }
            
            Some(Instance{ptr: p})
        }
    }

    /// Try to start a user interface for the libvlc instance.
    pub fn add_intf(&self, name: &str) -> Result<(), ()> {
        let cstr = to_cstr(name);

        let result = unsafe{
            ffi::libvlc_add_intf(self.ptr, cstr.as_ptr())
        };

        if result == 0 { Ok(()) }
        else { Err(()) }
    }

    /// Sets the application name.
    /// LibVLC passes this as the user agent string when a protocol requires it.
    pub fn set_user_agent(&self, name: &str, http: &str) {
        unsafe{
            ffi::libvlc_set_user_agent(
                self.ptr, to_cstr(name).as_ptr(), to_cstr(http).as_ptr());
        }
    }

    /// Waits until an interface causes the instance to exit.
    pub fn wait(&self) {
        unsafe{ ffi::libvlc_wait(self.ptr) };
    }

    /// Sets some meta-information about the application.
    pub fn set_app_id(&self, id: &str, version: &str, icon: &str) {
        unsafe{
            ffi::libvlc_set_app_id(
                self.ptr, to_cstr(id).as_ptr(), to_cstr(version).as_ptr(), to_cstr(icon).as_ptr());
        }
    }

    /// Returns a list of audio filters that are available.
    pub fn audio_filter_list_get(&self) -> Option<ModuleDescriptionList> {
        unsafe{
            let p = ffi::libvlc_audio_filter_list_get(self.ptr);
            if p.is_null() { None }
            else { Some(ModuleDescriptionList{ptr: p}) }
        }
    }

    /// Returns a list of video filters that are available.
    pub fn video_filter_list_get(&self) -> Option<ModuleDescriptionList> {
        unsafe{
            let p = ffi::libvlc_video_filter_list_get(self.ptr);
            if p.is_null() { None }
            else { Some(ModuleDescriptionList{ptr: p}) }
        }
    }

    /// Set logging callback
    pub fn set_log<F: Fn(LogLevel, Log, Cow<str>) + Send + 'static>(&self, f: F) {
        let cb: Box<Box<Fn(LogLevel, Log, Cow<str>) + Send + 'static>> = Box::new(Box::new(f));
        
        unsafe{
            ffi::libvlc_log_set(self.ptr, logging_cb, Box::into_raw(cb) as *mut _);
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe{
            ffi::libvlc_release(self.ptr);
        }
    }
}

extern "C" {
    fn vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, arg: ffi::va_list);
}
const BUF_SIZE: usize = 1024; // Write log message to the buffer by vsnprintf.
unsafe extern "C" fn logging_cb(
    data: *mut c_void, level: c_int, ctx: *const ffi::libvlc_log_t, fmt: *const c_char, args: ffi::va_list) {

    let f: &Box<Fn(LogLevel, Log, Cow<str>) + Send + 'static> = ::std::mem::transmute(data);
    let mut buf: [c_char; BUF_SIZE] = [0; BUF_SIZE];

    vsnprintf(buf.as_mut_ptr(), BUF_SIZE, fmt, args);

    f(::std::mem::transmute(level), Log{ptr: ctx}, from_cstr_ref(buf.as_ptr()).unwrap());
}

/// List of module description.
pub struct ModuleDescriptionList {
    ptr: *mut ffi::libvlc_module_description_t,
}

impl Drop for ModuleDescriptionList {
    fn drop(&mut self) {
        unsafe{ ffi::libvlc_module_description_list_release(self.ptr) };
    }
}

impl<'a> IntoIterator for &'a ModuleDescriptionList {
    type Item = ModuleDescriptionRef<'a>;
    type IntoIter = ModuleDescriptionListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ModuleDescriptionListIter{ptr: self.ptr, _phantomdata: PhantomData}
    }
}

pub struct ModuleDescriptionListIter<'a> {
    ptr: *mut ffi::libvlc_module_description_t,
    _phantomdata: PhantomData<&'a ffi::libvlc_module_description_t>,
}

/// Description of a module.
/// The strings are owned.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ModuleDescription {
    pub name:      Option<String>,
    pub shortname: Option<String>,
    pub longname:  Option<String>,
    pub help:      Option<String>,
}

/// Description of a module. 
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ModuleDescriptionRef<'a> {
    pub name:      Option<Cow<'a, str>>,
    pub shortname: Option<Cow<'a, str>>,
    pub longname:  Option<Cow<'a, str>>,
    pub help:      Option<Cow<'a, str>>,
}

impl<'a> Iterator for ModuleDescriptionListIter<'a> {
    type Item = ModuleDescriptionRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe{
            if self.ptr.is_null() {
                return None;
            }
            let p = self.ptr;
            self.ptr = (*p).p_next;
            Some(ModuleDescriptionRef{
                name:      from_cstr_ref((*p).psz_name),
                shortname: from_cstr_ref((*p).psz_shortname),
                longname:  from_cstr_ref((*p).psz_longname),
                help:      from_cstr_ref((*p).psz_help),
            })
        }
    }
}

impl<'a> ModuleDescriptionRef<'a> {
    /// Convert to owned strings.
    pub fn into_owned(&'a self) -> ModuleDescription {
        ModuleDescription {
            name:      self.name     .as_ref().map(|s| s.clone().into_owned()),
            shortname: self.shortname.as_ref().map(|s| s.clone().into_owned()),
            longname:  self.name     .as_ref().map(|s| s.clone().into_owned()),
            help:      self.shortname.as_ref().map(|s| s.clone().into_owned()),
        }
    }
}

pub fn errmsg() -> Option<String> {
    unsafe{ from_cstr(ffi::libvlc_errmsg()) }
}

pub fn clearerr() {
    unsafe{ ffi::libvlc_clearerr() };
}

#[derive(Clone, Debug)]
pub enum Event {
    MediaMetaChanged(Meta),
    MediaSubItemAdded,
    MediaDurationChanged(i64),
    MediaParsedChanged(i32),
    MediaFreed,
    MediaStateChanged(State),
    MediaSubItemTreeAdded,
    
    MediaPlayerMediaChanged,
    MediaPlayerNothingSpecial,
    MediaPlayerOpening,
    MediaPlayerBuffering(f32),
    MediaPlayerPlaying,
    MediaPlayerPaused,
    MediaPlayerStopped,
    MediaPlayerForward,
    MediaPlayerBackward,
    MediaPlayerEndReached,
    MediaPlayerEncounteredError,
    MediaPlayerTimeChanged,
    MediaPlayerPositionChanged(f32),
    MediaPlayerSeekableChanged,
    MediaPlayerPausableChanged,
    MediaPlayerTitleChanged,
    MediaPlayerSnapshotTaken,
    MediaPlayerLengthChanged,
    MediaPlayerVout,
    MediaPlayerScrambledChanged,

    MediaListItemAdded,
    MediaListWillAddItem,
    MediaListItemDeleted,
    MediaListWillDeleteItem,

    MediaListViewItemAdded,
    MediaListViewWillAddItem,
    MediaListViewItemDeleted,
    MediaListViewWillDeleteItem,

    MediaListPlayerPlayed,
    MediaListPlayerNextItemSet,
    MediaListPlayerStopped,

    MediaDiscovererStarted,
    MediaDiscovererEnded,

    VlmMediaAdded,
    VlmMediaRemoved,
    VlmMediaChanged,
    VlmMediaInstanceStarted,
    VlmMediaInstanceStopped,
    VlmMediaInstanceStatusInit,
    VlmMediaInstanceStatusOpening,
    VlmMediaInstanceStatusPlaying,
    VlmMediaInstanceStatusPause,
    VlmMediaInstanceStatusEnd,
    VlmMediaInstanceStatusError
}

pub struct EventManager<'a> {
    pub ptr: *mut ffi::libvlc_event_manager_t,
    pub _phantomdata: ::std::marker::PhantomData<&'a ffi::libvlc_event_manager_t>,
}

impl<'a> EventManager<'a> {
    pub fn attach<F>(&self, event_type: EventType, callback: F) -> Result<(), ()>
        where F: Fn(Event, VLCObject) + Send + 'static
    {
        // Explicit type annotation is needed
        let callback: Box<Box<Fn(Event, VLCObject) + Send + 'static>> =
            Box::new(Box::new(callback));
        
        let result = unsafe{
            ffi::libvlc_event_attach(
                self.ptr, event_type as i32, event_manager_callback,
                Box::into_raw(callback) as *mut c_void)
        };

        if result == 0 {
            Ok(())
        }else{
            Err(())
        }
    }
}

unsafe extern "C" fn event_manager_callback(pe: *const ffi::libvlc_event_t, data: *mut c_void) {
    let f: &Box<Fn(Event, VLCObject) + Send + 'static> = ::std::mem::transmute(data);

    f(conv_event(pe), VLCObject{_ptr: (*pe).p_obj});
}

// Convert c-style libvlc_event_t to Event
fn conv_event(pe: *const ffi::libvlc_event_t) -> Event {
    let event_type: EventType = unsafe{ ::std::mem::transmute((*pe)._type) };
    
    match event_type {
        EventType::MediaMetaChanged => {
            unsafe{
                let p = ffi::libvlc_event_t_union::get_media_meta_changed(pe);
                Event::MediaMetaChanged((*p).meta_type)
            }
        },
        EventType::MediaSubItemAdded => {
            Event::MediaSubItemAdded
        },
        EventType::MediaDurationChanged => {
            unsafe{
                let p = ffi::libvlc_event_t_union::get_media_duration_changed(pe);
                Event::MediaDurationChanged((*p).new_duration)
            }
        },
        EventType::MediaParsedChanged => {
            unsafe{
                let p = ffi::libvlc_event_t_union::get_media_parsed_changed(pe);
                Event::MediaParsedChanged((*p).new_status)
            }
        },
        EventType::MediaFreed => {
            Event::MediaFreed
        },
        EventType::MediaStateChanged => {
            unsafe{
                let p = ffi::libvlc_event_t_union::get_media_state_changed(pe);
                Event::MediaStateChanged((*p).new_state)
            }
        },
        EventType::MediaSubItemTreeAdded => {
            Event::MediaSubItemTreeAdded
        },
        EventType::MediaPlayerMediaChanged => {
            Event::MediaPlayerMediaChanged
        },
        EventType::MediaPlayerNothingSpecial => {
            Event::MediaPlayerNothingSpecial
        },
        EventType::MediaPlayerOpening => {
            Event::MediaPlayerOpening
        },
        EventType::MediaPlayerBuffering => {
            unsafe{
                let p = ffi::libvlc_event_t_union::get_media_player_buffering(pe);
                Event::MediaPlayerBuffering((*p).new_cache)
            }
        },
        EventType::MediaPlayerPlaying => {
            Event::MediaPlayerPlaying
        },
        EventType::MediaPlayerPaused => {
            Event::MediaPlayerPaused
        },
        EventType::MediaPlayerStopped => {
            Event::MediaPlayerStopped
        },
        EventType::MediaPlayerForward => {
            Event::MediaPlayerForward
        },
        EventType::MediaPlayerBackward => {
            Event::MediaPlayerBackward
        },
        EventType::MediaPlayerEndReached => {
            Event::MediaPlayerEndReached
        },
        EventType::MediaPlayerEncounteredError => {
            Event::MediaPlayerEncounteredError
        },
        EventType::MediaPlayerTimeChanged => {
            Event::MediaPlayerTimeChanged
        },
        EventType::MediaPlayerPositionChanged => {
            unsafe{
                let p = ffi::libvlc_event_t_union::get_media_player_position_changed(pe);
                Event::MediaPlayerPositionChanged((*p).new_position)
            }
        },
        EventType::MediaPlayerSeekableChanged => {
            Event::MediaPlayerSeekableChanged
        },
        EventType::MediaPlayerPausableChanged => {
            Event::MediaPlayerPausableChanged
        },
        EventType::MediaPlayerTitleChanged => {
            Event::MediaPlayerTitleChanged
        },
        EventType::MediaPlayerSnapshotTaken => {
            Event::MediaPlayerSnapshotTaken
        },
        EventType::MediaPlayerLengthChanged => {
            Event::MediaPlayerLengthChanged
        },
        EventType::MediaPlayerVout => {
            Event::MediaPlayerVout
        },
        EventType::MediaPlayerScrambledChanged => {
            Event::MediaPlayerScrambledChanged
        },
        EventType::MediaListItemAdded => {
            Event::MediaListItemAdded
        },
        EventType::MediaListWillAddItem => {
            Event::MediaListWillAddItem
        },
        EventType::MediaListItemDeleted => {
            Event::MediaListItemDeleted
        },
        EventType::MediaListWillDeleteItem => {
            Event::MediaListWillDeleteItem
        },
        EventType::MediaListViewItemAdded => {
            Event::MediaListViewItemAdded
        },
        EventType::MediaListViewWillAddItem => {
            Event::MediaListViewWillAddItem
        },
        EventType::MediaListViewItemDeleted => {
            Event::MediaListViewItemDeleted
        },
        EventType::MediaListViewWillDeleteItem => {
            Event::MediaListViewWillDeleteItem
        },
        EventType::MediaListPlayerPlayed => {
            Event::MediaListPlayerPlayed
        },
        EventType::MediaListPlayerNextItemSet => {
            Event::MediaListPlayerNextItemSet
        },
        EventType::MediaListPlayerStopped => {
            Event::MediaListPlayerStopped
        },
        EventType::MediaDiscovererStarted => {
            Event::MediaDiscovererStarted
        },
        EventType::MediaDiscovererEnded => {
            Event::MediaDiscovererEnded
        },
        EventType::VlmMediaAdded => {
            Event::VlmMediaAdded
        },
        EventType::VlmMediaRemoved => {
            Event::VlmMediaRemoved
        },
        EventType::VlmMediaChanged => {
            Event::VlmMediaChanged
        },
        EventType::VlmMediaInstanceStarted => {
            Event::VlmMediaInstanceStarted
        },
        EventType::VlmMediaInstanceStopped => {
            Event::VlmMediaInstanceStopped
        },
        EventType::VlmMediaInstanceStatusInit => {
            Event::VlmMediaInstanceStatusInit
        },
        EventType::VlmMediaInstanceStatusOpening => {
            Event::VlmMediaInstanceStatusOpening
        },
        EventType::VlmMediaInstanceStatusPlaying => {
            Event::VlmMediaInstanceStatusPlaying
        },
        EventType::VlmMediaInstanceStatusPause => {
            Event::VlmMediaInstanceStatusPause
        },
        EventType::VlmMediaInstanceStatusEnd => {
            Event::VlmMediaInstanceStatusEnd
        },
        EventType::VlmMediaInstanceStatusError => {
            Event::VlmMediaInstanceStatusError
        },
    }
}

pub struct VLCObject {
    _ptr: *mut c_void,
}

pub struct Log {
    pub ptr: *const ffi::libvlc_log_t
}

