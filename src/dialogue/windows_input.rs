use std::{io, mem};
use dialoguer::console::Key;
use encode_unicode::CharExt;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Console::{GetNumberOfConsoleInputEvents, GetStdHandle, INPUT_RECORD, KEY_EVENT, KEY_EVENT_RECORD, ReadConsoleInputW, STD_INPUT_HANDLE};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

pub fn try_read_single_key() -> io::Result<Option<Key>> {
    if get_key_event_count()? == 0 {
        return Ok(None);
    }

    let key_event_option = try_read_key_event()?;

    let key_event = match key_event_option {
        Some(e) => e,
        None => return Ok(None)
    };

    let unicode_char = unsafe { key_event.uChar.UnicodeChar };
    if unicode_char == 0 {
        Ok(Some(key_from_key_code(key_event.wVirtualKeyCode)))
    } else {
        // This is a unicode character, in utf-16. Try to decode it by itself.
        match char::from_utf16_tuple((unicode_char, None)) {
            Ok(c) => {
                // Maintain backward compatibility. The previous implementation (_getwch()) would return
                // a special keycode for `Enter`, while ReadConsoleInputW() prefers to use '\r'.
                if c == '\r' {
                    Ok(Some(Key::Enter))
                } else if c == '\x08' {
                    Ok(Some(Key::Backspace))
                } else if c == '\x1B' {
                    Ok(Some(Key::Escape))
                } else {
                    Ok(Some(Key::Char(c)))
                }
            }
            // This is part of a surrogate pair. Try to read the second half.
            Err(encode_unicode::error::Utf16TupleError::MissingSecond) => {
                // Confirm that there is a next character to read.
                if get_key_event_count()? == 0 {
                    let message = format!(
                        "Read invlid utf16 {}: {}",
                        unicode_char,
                        encode_unicode::error::Utf16TupleError::MissingSecond
                    );
                    return Err(io::Error::new(io::ErrorKind::InvalidData, message));
                }

                // Read the next character.
                let next_event = match try_read_key_event()? {
                    Some(key) => key,
                    None => return Err(io::Error::new(io::ErrorKind::InvalidData, "Expected as econd utf16 pair element"))
                };

                let next_surrogate = unsafe { next_event.uChar.UnicodeChar };

                // Attempt to decode it.
                match char::from_utf16_tuple((unicode_char, Some(next_surrogate))) {
                    Ok(c) => Ok(Some(Key::Char(c))),

                    // Return an InvalidData error. This is the recommended value for UTF-related I/O errors.
                    // (This error is given when reading a non-UTF8 file into a String, for example.)
                    Err(e) => {
                        let message = format!(
                            "Read invalid surrogate pair ({}, {}): {}",
                            unicode_char, next_surrogate, e
                        );
                        Err(io::Error::new(io::ErrorKind::InvalidData, message))
                    }
                }
            }

            // Return an InvalidData error. This is the recommended value for UTF-related I/O errors.
            // (This error is given when reading a non-UTF8 file into a String, for example.)
            Err(e) => {
                let message = format!("Read invalid utf16 {}: {}", unicode_char, e);
                Err(io::Error::new(io::ErrorKind::InvalidData, message))
            }
        }
    }
}

fn try_read_key_event() -> io::Result<Option<KEY_EVENT_RECORD>> {
    let handle = get_stdin_handle()?;
    let mut buffer: INPUT_RECORD = unsafe { mem::zeroed() };

    let mut events_read: u32 = unsafe { mem::zeroed() };

    let mut key_event: KEY_EVENT_RECORD;
    loop {
        let success = unsafe { ReadConsoleInputW(handle, &mut buffer, 1, &mut events_read) };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        if events_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "ReadConsoleInput returned no events, instead of waiting for an event",
            ));
        }

        if events_read == 1 && buffer.EventType != KEY_EVENT as u16 {
            // This isn't a key event; ignore it.
            return Ok(None);
        }

        key_event = unsafe { mem::transmute(buffer.Event) };

        if key_event.bKeyDown == 0 {
            // This is a key being released; ignore it.
            return Ok(None);
        }

        return Ok(Some(key_event));
    }
}

fn get_stdin_handle() -> io::Result<windows_sys::Win32::Foundation::HANDLE> {
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(handle)
    }
}

fn get_key_event_count() -> io::Result<u32> {
    let handle = get_stdin_handle()?;
    let mut event_count: u32 = unsafe { mem::zeroed() };

    let success = unsafe { GetNumberOfConsoleInputEvents(handle, &mut event_count) };
    if success == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(event_count)
    }
}


pub fn key_from_key_code(code: VIRTUAL_KEY) -> Key {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse;

    match code {
        KeyboardAndMouse::VK_LEFT => Key::ArrowLeft,
        KeyboardAndMouse::VK_RIGHT => Key::ArrowRight,
        KeyboardAndMouse::VK_UP => Key::ArrowUp,
        KeyboardAndMouse::VK_DOWN => Key::ArrowDown,
        KeyboardAndMouse::VK_RETURN => Key::Enter,
        KeyboardAndMouse::VK_ESCAPE => Key::Escape,
        KeyboardAndMouse::VK_BACK => Key::Backspace,
        KeyboardAndMouse::VK_TAB => Key::Tab,
        KeyboardAndMouse::VK_HOME => Key::Home,
        KeyboardAndMouse::VK_END => Key::End,
        KeyboardAndMouse::VK_DELETE => Key::Del,
        KeyboardAndMouse::VK_SHIFT => Key::Shift,
        KeyboardAndMouse::VK_MENU => Key::Alt,
        _ => Key::Unknown,
    }
}
