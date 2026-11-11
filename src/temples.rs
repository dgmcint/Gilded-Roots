//! Latter-day Saint ordinances: the temple list and the ordinance statuses used in GEDCOM files.
//!
//! The codes are the five-letter temple codes GEDCOM files use in `TEMP` lines (as written by PAF,
//! FamilySearch and other programs). Codes not on this list can still be typed and are kept.

/// (GEDCOM code, temple) in code order.
pub(crate) const TEMPLES: &[(&str, &str)] = &[
    ("ABA", "Aba, Nigeria"),
    ("ACCRA", "Accra, Ghana"),
    ("ADELA", "Adelaide, Australia"),
    ("ALBER", "Cardston, Alberta, Canada"),
    ("ALBUQ", "Albuquerque, New Mexico, USA"),
    ("ANCHO", "Anchorage, Alaska, USA"),
    ("APIA", "Apia, Samoa"),
    ("AREQU", "Arequipa, Peru"),
    ("ARIZO", "Mesa, Arizona, USA"),
    ("ASUNC", "Asunción, Paraguay"),
    ("ATLAN", "Atlanta, Georgia, USA"),
    ("BAIRE", "Buenos Aires, Argentina"),
    ("BARRA", "Barranquilla, Colombia"),
    ("BILLI", "Billings, Montana, USA"),
    ("BIRMI", "Birmingham, Alabama, USA"),
    ("BISMA", "Bismarck, North Dakota, USA"),
    ("BOGOT", "Bogotá, Colombia"),
    ("BOISE", "Boise, Idaho, USA"),
    ("BOSTO", "Boston, Massachusetts, USA"),
    ("BOUNT", "Bountiful, Utah, USA"),
    ("BRIGH", "Brigham City, Utah, USA"),
    ("BRISB", "Brisbane, Australia"),
    ("BROUG", "Baton Rouge, Louisiana, USA"),
    ("CALGA", "Calgary, Alberta, Canada"),
    ("CAMPI", "Campinas, Brazil"),
    ("CARAC", "Caracas, Venezuela"),
    ("CEBUP", "Cebu City, Philippines"),
    ("CEDAR", "Cedar City, Utah, USA"),
    ("CHICA", "Chicago, Illinois, USA"),
    ("CIUJU", "Ciudad Juárez, Mexico"),
    ("COCHA", "Cochabamba, Bolivia"),
    ("COLJU", "Colonia Juárez, Mexico"),
    ("COLSC", "Columbia, South Carolina, USA"),
    ("COLUM", "Columbus, Ohio, USA"),
    ("CONCE", "Concepción, Chile"),
    ("COPEN", "Copenhagen, Denmark"),
    ("CORDO", "Córdoba, Argentina"),
    ("CRIVE", "Columbia River, Washington, USA"),
    ("CURIT", "Curitiba, Brazil"),
    ("DALLA", "Dallas, Texas, USA"),
    ("DENVE", "Denver, Colorado, USA"),
    ("DETRO", "Detroit, Michigan, USA"),
    ("DRAPE", "Draper, Utah, USA"),
    ("EDMON", "Edmonton, Alberta, Canada"),
    ("EHOUS", "Endowment House"),
    ("FORTA", "Fortaleza, Brazil"),
    ("FORTC", "Fort Collins, Colorado, USA"),
    ("FORTL", "Fort Lauderdale, Florida, USA"),
    ("FRANK", "Frankfurt, Germany"),
    ("FREIB", "Freiberg, Germany"),
    ("FRESN", "Fresno, California, USA"),
    ("FUKUO", "Fukuoka, Japan"),
    ("GILAV", "Gila Valley, Arizona, USA"),
    ("GILBE", "Gilbert, Arizona, USA"),
    ("GUADA", "Guadalajara, Mexico"),
    ("GUATE", "Guatemala City, Guatemala"),
    ("GUAYA", "Guayaquil, Ecuador"),
    ("HAGUE", "The Hague, Netherlands"),
    ("HALIF", "Halifax, Nova Scotia, Canada"),
    ("HARTF", "Hartford, Connecticut, USA"),
    ("HAWAI", "Laie, Hawaii, USA"),
    ("HELSI", "Helsinki, Finland"),
    ("HERMO", "Hermosillo, Mexico"),
    ("HKONG", "Hong Kong"),
    ("HOUST", "Houston, Texas, USA"),
    ("IFALL", "Idaho Falls, Idaho, USA"),
    ("INDIA", "Indianapolis, Indiana, USA"),
    ("JOHAN", "Johannesburg, South Africa"),
    ("JRIVE", "Jordan River, Utah, USA"),
    ("KANSA", "Kansas City, Missouri, USA"),
    ("KINSH", "Kinshasa, DR Congo"),
    ("KIRTL", "Kirtland, Ohio, USA"),
    ("KONA", "Kona, Hawaii, USA"),
    ("KYIV", "Kyiv, Ukraine"),
    ("LANGE", "Los Angeles, California, USA"),
    ("LIMA", "Lima, Peru"),
    ("LISBO", "Lisbon, Portugal"),
    ("LOGAN", "Logan, Utah, USA"),
    ("LONDO", "London, England"),
    ("LOUIS", "Louisville, Kentucky, USA"),
    ("LUBBO", "Lubbock, Texas, USA"),
    ("LVEGA", "Las Vegas, Nevada, USA"),
    ("MADRI", "Madrid, Spain"),
    ("MANAU", "Manaus, Brazil"),
    ("MANHA", "Manhattan, New York, USA"),
    ("MANIL", "Manila, Philippines"),
    ("MANTI", "Manti, Utah, USA"),
    ("MEDFO", "Medford, Oregon, USA"),
    ("MELBO", "Melbourne, Australia"),
    ("MEMPH", "Memphis, Tennessee, USA"),
    ("MERID", "Mérida, Mexico"),
    ("MEXIC", "Mexico City, Mexico"),
    ("MIDAH", "Meridian, Idaho, USA"),
    ("MNTVD", "Montevideo, Uruguay"),
    ("MONTE", "Monterrey, Mexico"),
    ("MONTI", "Monticello, Utah, USA"),
    ("MONTR", "Montreal, Quebec, Canada"),
    ("MTIMP", "Mount Timpanogos, Utah, USA"),
    ("NASHV", "Nashville, Tennessee, USA"),
    ("NAUV2", "Nauvoo Illinois (rebuilt), USA"),
    ("NAUVO", "Nauvoo Illinois (original), USA"),
    ("NBEAC", "Newport Beach, California, USA"),
    ("NUKUA", "Nukuʻalofa, Tonga"),
    ("NYORK", "New York, New York, USA"),
    ("NZEAL", "Hamilton, New Zealand"),
    ("OAKLA", "Oakland, California, USA"),
    ("OAXAC", "Oaxaca, Mexico"),
    ("OGDEN", "Ogden, Utah, USA"),
    ("OKLAH", "Oklahoma City, Oklahoma, USA"),
    ("OMOUN", "Oquirrh Mountain, Utah, USA"),
    ("OQUIR", "Oquirrh Mountain, Utah, USA"),
    ("ORLAN", "Orlando, Florida, USA"),
    ("PALEG", "Porto Alegre, Brazil"),
    ("PALMY", "Palmyra, New York, USA"),
    ("PANAM", "Panama City, Panama"),
    ("PAPEE", "Papeete, Tahiti"),
    ("PARIS", "Paris, France"),
    ("PAYSO", "Payson, Utah, USA"),
    ("PERTH", "Perth, Australia"),
    ("PHILA", "Philadelphia, Pennsylvania, USA"),
    ("PHOEN", "Phoenix, Arizona, USA"),
    ("POFFI", "President's Office"),
    ("PORTA", "Port-au-Prince, Haiti"),
    ("PORTL", "Portland, Oregon, USA"),
    ("PREST", "Preston, England"),
    ("PROCC", "Provo City Center, Utah, USA"),
    ("PROVO", "Provo, Utah, USA"),
    ("QUETZ", "Quetzaltenango, Guatemala"),
    ("RALEI", "Raleigh, North Carolina, USA"),
    ("RECIF", "Recife, Brazil"),
    ("REDLA", "Redlands, California, USA"),
    ("REGIN", "Regina, Saskatchewan, Canada"),
    ("RENO", "Reno, Nevada, USA"),
    ("REXBU", "Rexburg, Idaho, USA"),
    ("ROME", "Rome, Italy"),
    ("SACRA", "Sacramento, California, USA"),
    ("SANSA", "San Salvador, El Salvador"),
    ("SANTI", "Santiago, Chile"),
    ("SANTO", "San Antonio, Texas, USA"),
    ("SAPPO", "Sapporo, Japan"),
    ("SDIEG", "San Diego, California, USA"),
    ("SDOMI", "Santo Domingo, Dominican Republic"),
    ("SEATT", "Seattle, Washington, USA"),
    ("SEOUL", "Seoul, Korea"),
    ("SGEOR", "St. George, Utah, USA"),
    ("SJOSE", "San José, Costa Rica"),
    ("SLAKE", "Salt Lake City, Utah, USA"),
    ("SLOUI", "St. Louis, Missouri, USA"),
    ("SNOWF", "Snowflake, Arizona, USA"),
    ("SPAUL", "São Paulo, Brazil"),
    ("SPMIN", "St. Paul, Minnesota, USA"),
    ("SPOKA", "Spokane, Washington, USA"),
    ("SSALV", "San Salvador, El Salvador"),
    ("STARV", "Star Valley, Wyoming, USA"),
    ("STOCK", "Stockholm, Sweden"),
    ("SUVA", "Suva, Fiji"),
    ("SWISS", "Bern, Switzerland"),
    ("SYDNE", "Sydney, Australia"),
    ("TAIPE", "Taipei, Taiwan"),
    ("TAMPI", "Tampico, Mexico"),
    ("TEGUC", "Tegucigalpa, Honduras"),
    ("TFALL", "Twin Falls, Idaho, USA"),
    ("TGUTI", "Tuxtla Gutiérrez, Mexico"),
    ("TIJUA", "Tijuana, Mexico"),
    ("TOKYO", "Tokyo, Japan"),
    ("TORNO", "Toronto, Ontario, Canada"),
    ("TORON", "Toronto, Ontario, Canada"),
    ("TRUJI", "Trujillo, Peru"),
    ("TWINF", "Twin Falls, Idaho, USA"),
    ("VANCO", "Vancouver, British Columbia, Canada"),
    ("VERAC", "Veracruz, Mexico"),
    ("VERNA", "Vernal, Utah, USA"),
    ("VILLA", "Villahermosa, Mexico"),
    ("WASHI", "Washington, D.C., USA"),
    ("WINTE", "Winter Quarters, Nebraska, USA"),
];

/// Older or other codes found in files (PAF's two-letter codes, early spellings, and places that
/// are not temples), with the temple or place they stand for. Shown by name, never offered in the list.
pub(crate) const OTHER_CODES: &[(&str, &str)] = &[
    ("AL", "Cardston, Alberta, Canada"), ("ALBR", "Cardston, Alberta, Canada"), ("AP", "Apia, Samoa"), ("AT", "Atlanta, Georgia, USA"),
    ("AZ", "Mesa, Arizona, USA"), ("BA", "Buenos Aires, Argentina"), ("BG", "Bogotá, Colombia"), ("BO", "Boise, Idaho, USA"),
    ("CEBU", "Cebu City, Philippines"), ("CH", "Chicago, Illinois, USA"), ("DA", "Dallas, Texas, USA"), ("DV", "Denver, Colorado, USA"),
    ("EH", "Endowment House, Salt Lake City"), ("FD", "Freiberg, Germany"), ("FR", "Frankfurt, Germany"), ("GA", "Guatemala City, Guatemala"),
    ("GU", "Guatemala City, Guatemala"), ("GVALL", "Gila Valley, Arizona, USA"), ("GY", "Guayaquil, Ecuador"), ("HA", "Laie, Hawaii, USA"),
    ("HARRI", "Harrison, New York, USA"), ("HO", "Historian's Office"), ("HOFFI", "Historian's Office"), ("IF", "Idaho Falls, Idaho, USA"),
    ("JO", "Johannesburg, South Africa"), ("JR", "Jordan River, Utah, USA"), ("KIEV", "Kyiv, Ukraine"), ("LA", "Los Angeles, California, USA"),
    ("LD", "London, England"), ("LG", "Logan, Utah, USA"), ("LI", "Lima, Peru"), ("LV", "Las Vegas, Nevada, USA"), ("MA", "Manila, Philippines"),
    ("MT", "Manti, Utah, USA"), ("MX", "Mexico City, Mexico"), ("NV", "Nauvoo, Illinois, USA"), ("NZ", "Hamilton, New Zealand"),
    ("OG", "Ogden, Utah, USA"), ("OK", "Oakland, California, USA"), ("OTHER", "Other"), ("PO", "President's Office"),
    ("PT", "Portland, Oregon, USA"), ("PV", "Provo, Utah, USA"), ("ROMEI", "Rome, Italy"), ("SA", "San Diego, California, USA"),
    ("SAMOA", "Apia, Samoa"), ("SD", "Sydney, Australia"), ("SE", "Seattle, Washington, USA"), ("SG", "St. George, Utah, USA"),
    ("SL", "Salt Lake City, Utah, USA"), ("SN", "Santiago, Chile"), ("SO", "Seoul, South Korea"), ("SP", "São Paulo, Brazil"),
    ("ST", "Stockholm, Sweden"), ("SW", "Bern, Switzerland"), ("TA", "Papeete, Tahiti"), ("TG", "Nukuʻalofa, Tonga"),
    ("TK", "Tokyo, Japan"), ("TP", "Taipei, Taiwan"), ("TR", "Toronto, Ontario, Canada"),
];

/// The ordinance status words GEDCOM files use in `STAT` lines, with what each means.
pub(crate) const ORD_STATUS: &[(&str, &str)] = &[
    ("COMPLETED", "Completed"),
    ("SUBMITTED", "Submitted"),
    ("CLEARED", "Cleared, not yet done"),
    ("UNCLEARED", "Not cleared"),
    ("QUALIFIED", "Qualified"),
    ("BIC", "Born in the covenant"),
    ("CHILD", "Died before age 8"),
    ("INFANT", "Died before age 1"),
    ("STILLBORN", "Stillborn"),
    ("CANCELED", "Sealing canceled"),
    ("DNS", "Do not submit"),
    ("DNS/CAN", "Do not submit, sealing canceled"),
    ("EXCLUDED", "Excluded"),
    ("PRE-1970", "Done before 1970"),
];

/// The temple's name for a code, if the code is known.
pub(crate) fn temple_name(code: &str) -> Option<&'static str> {
    let c = code.trim().to_ascii_uppercase();
    TEMPLES.iter().chain(OTHER_CODES).find(|(k, _)| *k == c).map(|(_, n)| *n)
}

/// How a temple code reads on screen: "Logan, Utah, USA (LOGAN)", or the code itself.
pub(crate) fn temple_label(code: &str) -> String {
    match temple_name(code) {
        Some(n) => format!("{n} ({})", code.trim().to_ascii_uppercase()),
        None => code.trim().to_string(),
    }
}

/// How a status word reads on screen.
pub(crate) fn status_label(stat: &str) -> String {
    let s = stat.trim();
    ORD_STATUS.iter().find(|(k, _)| k.eq_ignore_ascii_case(s)).map(|(_, l)| l.to_string()).unwrap_or_else(|| s.to_string())
}

/// A list of temples to choose from, with a search box. Returns true when one was chosen.
pub(crate) fn temple_picker(ui: &mut eframe::egui::Ui, id: impl std::hash::Hash, code: &mut String, search: &mut String) -> bool {
    use eframe::egui;
    let shown = if code.trim().is_empty() { "Choose a temple".to_string() } else { temple_label(code) };
    let mut picked = false;
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(230.0).height(320.0).show_ui(ui, |ui| {
        let r = ui.add(egui::TextEdit::singleline(search).hint_text("Type a place or code").desired_width(200.0));
        if !r.has_focus() && !r.lost_focus() {
            r.request_focus();
        }
        let q = search.trim().to_lowercase();
        if ui.selectable_label(code.trim().is_empty(), "(none)").clicked() {
            code.clear();
            picked = true;
        }
        let mut by_name: Vec<&(&str, &str)> = TEMPLES.iter().collect();
        by_name.sort_by_key(|(_, n)| n.to_lowercase());
        for (c, n) in by_name {
            if !q.is_empty() && !n.to_lowercase().contains(&q) && !c.to_lowercase().contains(&q) {
                continue;
            }
            if ui.selectable_label(code.trim().eq_ignore_ascii_case(c), format!("{n}  ({c})")).clicked() {
                *code = c.to_string();
                picked = true;
            }
        }
    });
    picked
}

/// A list of ordinance statuses to choose from.
pub(crate) fn status_picker(ui: &mut eframe::egui::Ui, id: impl std::hash::Hash, stat: &mut String) {
    use eframe::egui;
    let shown = if stat.trim().is_empty() { "Status".to_string() } else { status_label(stat) };
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(170.0).show_ui(ui, |ui| {
        if ui.selectable_label(stat.trim().is_empty(), "(none)").clicked() {
            stat.clear();
        }
        for (k, l) in ORD_STATUS {
            if ui.selectable_label(stat.trim().eq_ignore_ascii_case(k), *l).clicked() {
                *stat = k.to_string();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temple_codes_are_unique_five_letters_or_fewer_and_in_order() {
        for w in TEMPLES.windows(2) {
            assert!(w[0].0 < w[1].0, "{} before {}", w[0].0, w[1].0);
        }
        assert!(TEMPLES.iter().all(|(c, _)| c.len() <= 5 && c.chars().all(|x| x.is_ascii_uppercase() || x.is_ascii_digit())));
        assert_eq!(temple_name("slake"), Some("Salt Lake City, Utah, USA"));
        assert!(OTHER_CODES.iter().all(|(c, _)| TEMPLES.iter().all(|(t, _)| t != c)), "an old code is also a current one");
        assert_eq!(temple_label("lg"), "Logan, Utah, USA (LG)");
        assert_eq!(temple_label("XYZZY"), "XYZZY");
        assert_eq!(status_label("bic"), "Born in the covenant");
    }
}
