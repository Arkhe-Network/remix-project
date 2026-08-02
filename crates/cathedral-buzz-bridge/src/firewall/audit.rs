use anyhow::Result;
use nostr::Event;

#[derive(Debug, PartialEq, Eq)]
pub enum Zone {
    Z0Theory,
    Z1Tools,
    Z2Continuous,
    Z3Discrete,
}

// Firewall Z0-Z3
pub fn validate_event_firewall(event: &Event, expected_pubkey: &nostr::PublicKey) -> Result<()> {
    // T10 — Valida assinatura primeiro
    event.verify()?;

    // Validar pubkey
    if event.pubkey != *expected_pubkey {
        return Err(anyhow::anyhow!("Invalid event publisher"));
    }

    // T8 — Binding criptográfico, verificar signature binding
    let mut has_translation_tag = false;
    let mut translation_hash = None;

    for tag in event.tags.iter() {
        let as_vec = tag.clone().to_vec();
        if as_vec.len() >= 2 && as_vec[0] == "translation" {
            has_translation_tag = true;
            if as_vec.len() >= 3 {
                translation_hash = Some(as_vec[2].clone());
            }
        }
    }

    // Se houver uma tradução, deve ter um hash válido atestando (criptograficamente amarrado no ID do evento)
    if has_translation_tag {
        if translation_hash.is_none() {
            return Err(anyhow::anyhow!("Translation tag missing cryptographic binding"));
        }
    }

    Ok(())
}
