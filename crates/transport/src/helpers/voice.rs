use grammers_client::media::Document;
use grammers_client::tl;

pub fn is_voice(doc: &Document) -> bool {
    let Some(tl::enums::Document::Document(d)) = doc.raw.document.as_ref() else {
        return false;
    };
    d.attributes.iter().any(|attr| match attr {
        tl::enums::DocumentAttribute::Audio(audio) => audio.voice,
        _ => false,
    })
}
