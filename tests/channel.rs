use std::thread;

use atomics_locks::one_shot_channel::{typed_channel, unsafe_channel};

#[test]
fn unsafe_channel() {
    let channel = unsafe_channel::Channel::new();
    let t = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            channel.send("hello world",);
            t.unpark();
        },);
        while !channel.is_ready() {
            thread::park()
        }
        assert_eq!(channel.receive(), "hello world");
    },)
}

#[test]
#[should_panic]
fn unsafe_channel_multiple_sends_panics() {
    let channel = unsafe_channel::Channel::new();
    let t = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            channel.send("hello world",);
            channel.send("hello world",);
            t.unpark();
        },);
        while !channel.is_ready() {
            thread::park()
        }
        assert_eq!(channel.receive(), "hello world");
    },)
}

#[test]
#[should_panic]
fn unsafe_channel_multiple_receives_panics() {
    let channel = unsafe_channel::Channel::new();
    let t = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            channel.send("hello world",);
            t.unpark();
        },);
        while !channel.is_ready() {
            thread::park()
        }
        assert_eq!(channel.receive(), "hello world");
        channel.receive();
    },)
}

#[test]
fn typed_channel() {
    let mut channel = typed_channel::Channel::new();
    thread::scope(|s| {
        let (sender, receiver,) = channel.split();
        s.spawn(move || {
            sender.send("hello world",);
        },);
        assert_eq!(receiver.receive(), "hello world");
    },)
}
