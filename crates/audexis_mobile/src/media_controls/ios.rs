use block::ConcreteBlock;
use cocoa::{
    base::{NO, YES, id, nil},
    foundation::NSAutoreleasePool,
};
use objc::{class, msg_send, sel, sel_impl};
use souvlaki::MediaControlEvent;
use std::sync::mpsc::SyncSender;

#[link(name = "MediaPlayer", kind = "framework")]
unsafe extern "C" {
    static MPNowPlayingInfoPropertyPlaybackRate: id;
    static MPNowPlayingInfoPropertyDefaultPlaybackRate: id;
    static MPNowPlayingInfoPropertyElapsedPlaybackTime: id;
    static MPNowPlayingInfoPropertyPlaybackQueueCount: id;
    static MPNowPlayingInfoPropertyPlaybackQueueIndex: id;
}

pub(super) struct Pool(id);
impl Pool {
    pub(super) fn new() -> Self {
        Self(unsafe { NSAutoreleasePool::new(nil) })
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        unsafe {
            self.0.drain();
        }
    }
}

pub(super) fn attach_stop(sender: SyncSender<MediaControlEvent>) {
    let _pool = Pool::new();
    unsafe {
        let center: id = msg_send![class!(MPRemoteCommandCenter), sharedCommandCenter];
        let command: id = msg_send![center, stopCommand];
        let handler = ConcreteBlock::new(move |_event: id| -> isize {
            if sender.try_send(MediaControlEvent::Stop).is_ok() {
                0
            } else {
                200
            }
        })
        .copy();
        let _: id = msg_send![command, addTargetWithHandler: &*handler];
        let _: () = msg_send![command, setEnabled: YES];
    }
}

pub(super) fn update(
    playing: bool,
    position_ms: u64,
    duration_ms: u64,
    index: Option<usize>,
    count: usize,
) {
    let _pool = Pool::new();
    unsafe {
        let center: id = msg_send![class!(MPNowPlayingInfoCenter), defaultCenter];
        let existing: id = msg_send![center, nowPlayingInfo];
        let info: id = msg_send![class!(NSMutableDictionary), dictionary];
        if existing != nil {
            let _: () = msg_send![info, addEntriesFromDictionary: existing];
        }
        for (key, value) in [
            (
                MPNowPlayingInfoPropertyPlaybackRate,
                if playing { 1.0 } else { 0.0 },
            ),
            (MPNowPlayingInfoPropertyDefaultPlaybackRate, 1.0),
            (
                MPNowPlayingInfoPropertyElapsedPlaybackTime,
                position_ms as f64 / 1000.0,
            ),
            (MPNowPlayingInfoPropertyPlaybackQueueCount, count as f64),
            (
                MPNowPlayingInfoPropertyPlaybackQueueIndex,
                index.unwrap_or(0) as f64,
            ),
        ] {
            let value: id = msg_send![class!(NSNumber), numberWithDouble: value];
            let _: () = msg_send![info, setObject: value forKey: key];
        }
        let _: () = msg_send![center, setNowPlayingInfo: info];
        let commands: id = msg_send![class!(MPRemoteCommandCenter), sharedCommandCenter];
        let next: id = msg_send![commands, nextTrackCommand];
        let previous: id = msg_send![commands, previousTrackCommand];
        let seek: id = msg_send![commands, changePlaybackPositionCommand];
        let _: () = msg_send![next, setEnabled: if index.is_some_and(|index| index + 1 < count) { YES } else { NO }];
        let _: () = msg_send![previous, setEnabled: if index.is_some() { YES } else { NO }];
        let _: () = msg_send![seek, setEnabled: if duration_ms > 0 { YES } else { NO }];
    }
}

pub(super) fn clear() {
    let _pool = Pool::new();
    unsafe {
        let center: id = msg_send![class!(MPNowPlayingInfoCenter), defaultCenter];
        let _: () = msg_send![center, setNowPlayingInfo: nil];
        let commands: id = msg_send![class!(MPRemoteCommandCenter), sharedCommandCenter];
        let stop: id = msg_send![commands, stopCommand];
        let _: () = msg_send![stop, setEnabled: NO];
        let _: () = msg_send![stop, removeTarget: nil];
    }
}
