//! Reports > Blank census forms: printable worksheets for copying down a census entry, one per
//! census year. The column headings follow the questions each government census asked, numbered
//! as on the original schedules where they had numbers; the wording and layout are our own.

use super::*;

/// One column on a form: its number on the original schedule ("" when unnumbered), a short
/// heading, and a relative width (1 = a tick or a small number, 7 = a name).
pub struct Col {
    pub n: &'static str,
    pub head: &'static str,
    pub w: u8,
}

/// Columns that share an overall heading ("Free white males").
pub struct Group {
    pub label: &'static str,
    pub cols: &'static [Col],
}

/// One printed sheet of a form. Very wide early schedules are split over several sheets, each
/// starting with the head of family so the sheets line up.
pub struct Sheet {
    pub title: &'static str,
    pub groups: &'static [Group],
}

pub struct Form {
    pub country: &'static str,
    pub year: i32,
    /// The official census day.
    pub day: &'static str,
    /// Where this form was used, when it was not the whole country ("Manitoba, Saskatchewan and Alberta").
    pub place: &'static str,
    /// One line per household (1790-1840) or one line per person.
    pub households: bool,
    pub sheets: &'static [Sheet],
    pub note: &'static str,
}

const fn c(n: &'static str, head: &'static str, w: u8) -> Col {
    Col { n, head, w }
}

const fn g(label: &'static str, cols: &'static [Col]) -> Group {
    Group { label, cols }
}

// Age brackets shared by several early censuses.
const AGES_1800: &[Col] = &[c("", "Under 10", 1), c("", "10-15", 1), c("", "16-25", 1), c("", "26-44", 1), c("", "45 & over", 1)];
const AGES_1830_WHITE: &[Col] = &[
    c("", "Under 5", 1),
    c("", "5-9", 1),
    c("", "10-14", 1),
    c("", "15-19", 1),
    c("", "20-29", 1),
    c("", "30-39", 1),
    c("", "40-49", 1),
    c("", "50-59", 1),
    c("", "60-69", 1),
    c("", "70-79", 1),
    c("", "80-89", 1),
    c("", "90-99", 1),
    c("", "100 & over", 1),
];
const AGES_1830_OTHER: &[Col] = &[c("", "Under 10", 1), c("", "10-23", 1), c("", "24-35", 1), c("", "36-54", 1), c("", "55-99", 1), c("", "100 & over", 1)];
const AGES_1820_OTHER: &[Col] = &[c("", "Under 14", 1), c("", "14-25", 1), c("", "26-44", 1), c("", "45 & over", 1)];
const HEAD: &[Col] = &[c("", "Name of head of family", 8)];
const NOTES: Group = g("", &[c("", "Notes", 5)]);

pub const US: &[Form] = &[
    Form {
        country: "United States",
        year: 1790,
        day: "2 August 1790",
        place: "",
        households: true,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", HEAD),
                g("Free white males", &[c("", "16 & over (incl. head)", 2), c("", "Under 16", 2)]),
                g("", &[c("", "Free white females (incl. head)", 2), c("", "All other free persons", 2), c("", "Slaves", 2)]),
                NOTES,
            ],
        }],
        note: "Only the head of household is named; everyone else is counted. Schedules for several states (including Delaware, Georgia, Kentucky, New Jersey, Tennessee and Virginia) were lost.",
    },
    Form {
        country: "United States",
        year: 1800,
        day: "4 August 1800",
        place: "",
        households: true,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", HEAD),
                g("Free white males", AGES_1800),
                g("Free white females", AGES_1800),
                g("", &[c("", "All other free persons (not Indians not taxed)", 2), c("", "Slaves", 1)]),
                NOTES,
            ],
        }],
        note: "Ages are in brackets: 10-15 means 10 up to (not including) 16.",
    },
    Form {
        country: "United States",
        year: 1810,
        day: "6 August 1810",
        place: "",
        households: true,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", HEAD),
                g("Free white males", AGES_1800),
                g("Free white females", AGES_1800),
                g("", &[c("", "All other free persons (not Indians not taxed)", 2), c("", "Slaves", 1)]),
                NOTES,
            ],
        }],
        note: "Ages are in brackets: 10-15 means 10 up to (not including) 16.",
    },
    Form {
        country: "United States",
        year: 1820,
        day: "7 August 1820",
        place: "",
        households: true,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", HEAD),
                g("Free white males", &[c("", "Under 10", 1), c("", "10-15", 1), c("", "16-18", 1), c("", "16-25", 1), c("", "26-44", 1), c("", "45 & over", 1)]),
                g("Free white females", AGES_1800),
                g("", &[c("", "Foreigners not naturalized", 1)]),
                g("Engaged in", &[c("", "Agriculture", 1), c("", "Commerce", 1), c("", "Manufactures", 1)]),
                g("Slaves: males", AGES_1820_OTHER),
                g("Slaves: females", AGES_1820_OTHER),
                g("Free colored: males", AGES_1820_OTHER),
                g("Free colored: females", AGES_1820_OTHER),
                g("", &[c("", "All other persons (not Indians not taxed)", 1)]),
                NOTES,
            ],
        }],
        note: "Males 16-18 are also counted in 16-25.",
    },
    Form {
        country: "United States",
        year: 1830,
        day: "1 June 1830",
        place: "",
        households: true,
        sheets: &[
            Sheet { title: "Free white persons", groups: &[g("", HEAD), g("Free white males", AGES_1830_WHITE), g("Free white females", AGES_1830_WHITE)] },
            Sheet {
                title: "Slaves, free colored persons and others",
                groups: &[
                    g("", HEAD),
                    g("Slaves: males", AGES_1830_OTHER),
                    g("Slaves: females", AGES_1830_OTHER),
                    g("Free colored: males", AGES_1830_OTHER),
                    g("Free colored: females", AGES_1830_OTHER),
                    g("", &[c("", "Total in household", 1)]),
                    g("White: deaf and dumb", &[c("", "Under 14", 1), c("", "14-24", 1), c("", "25 & over", 1)]),
                    g("White", &[c("", "Blind", 1), c("", "Aliens (not naturalized)", 1)]),
                    g("Slaves & colored: deaf and dumb", &[c("", "Under 14", 1), c("", "14-24", 1), c("", "25 & over", 1)]),
                    g("Slaves & colored", &[c("", "Blind", 1)]),
                ],
            },
        ],
        note: "Ages are in brackets: 5-9 means 5 up to (not including) 10. The words used for disabilities are those of the time.",
    },
    Form {
        country: "United States",
        year: 1840,
        day: "1 June 1840",
        place: "",
        households: true,
        sheets: &[
            Sheet { title: "Free white persons", groups: &[g("", HEAD), g("Free white males", AGES_1830_WHITE), g("Free white females", AGES_1830_WHITE)] },
            Sheet {
                title: "Free colored persons and slaves",
                groups: &[
                    g("", HEAD),
                    g("Free colored: males", AGES_1830_OTHER),
                    g("Free colored: females", AGES_1830_OTHER),
                    g("Slaves: males", AGES_1830_OTHER),
                    g("Slaves: females", AGES_1830_OTHER),
                    g("", &[c("", "Total in household", 1)]),
                ],
            },
            Sheet {
                title: "Work, pensioners, health and schooling",
                groups: &[
                    g("", HEAD),
                    g(
                        "Number employed in",
                        &[
                            c("", "Mining", 1),
                            c("", "Agriculture", 1),
                            c("", "Commerce", 1),
                            c("", "Manufactures & trades", 1),
                            c("", "Navigation of the ocean", 1),
                            c("", "Navigation of canals, lakes, rivers", 1),
                            c("", "Learned professions & engineers", 1),
                        ],
                    ),
                    g("Revolutionary or military pensioners", &[c("", "Name", 5), c("", "Age", 1)]),
                    g("White: deaf and dumb", &[c("", "Under 14", 1), c("", "14-24", 1), c("", "25 & over", 1)]),
                    g("White", &[c("", "Blind", 1), c("", "Insane: public charge", 1), c("", "Insane: private charge", 1)]),
                    g("Colored", &[c("", "Deaf and dumb", 1), c("", "Blind", 1), c("", "Insane: private charge", 1), c("", "Insane: public charge", 1)]),
                    g(
                        "Schools",
                        &[
                            c("", "Universities or colleges", 1),
                            c("", "Students", 1),
                            c("", "Academies & grammar schools", 1),
                            c("", "Students", 1),
                            c("", "Primary & common schools", 1),
                            c("", "Scholars", 1),
                            c("", "Scholars at public charge", 1),
                        ],
                    ),
                    g("", &[c("", "White persons over 20 who cannot read and write", 2)]),
                ],
            },
        ],
        note: "The pensioners column names veterans of the Revolution or other military service, with their ages. The words used for disabilities are those of the time.",
    },
    Form {
        country: "United States",
        year: 1850,
        day: "1 June 1850",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7)]),
                g("Description", &[c("4", "Age", 1), c("5", "Sex", 1), c("6", "Color", 1)]),
                g("", &[c("7", "Occupation (males over 15)", 4), c("8", "Value of real estate", 2), c("9", "Place of birth", 4)]),
                g("", &[c("10", "Married within the year", 1), c("11", "At school within the year", 1), c("12", "Over 20, cannot read & write", 1), c("13", "Deaf & dumb, blind, insane, idiotic, pauper or convict", 2)]),
                NOTES,
            ],
        }],
        note: "The first census to name every person. Slaves were counted on a separate slave schedule.",
    },
    Form {
        country: "United States",
        year: 1860,
        day: "1 June 1860",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7)]),
                g("Description", &[c("4", "Age", 1), c("5", "Sex", 1), c("6", "Color", 1)]),
                g("", &[c("7", "Occupation (over 15)", 4)]),
                g("Value of estate", &[c("8", "Real estate", 2), c("9", "Personal estate", 2)]),
                g("", &[c("10", "Place of birth", 4), c("11", "Married within the year", 1), c("12", "At school within the year", 1), c("13", "Over 20, cannot read & write", 1), c("14", "Deaf & dumb, blind, insane, idiotic, pauper or convict", 2)]),
                NOTES,
            ],
        }],
        note: "Slaves were counted on a separate slave schedule.",
    },
    Form {
        country: "United States",
        year: 1870,
        day: "1 June 1870",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7)]),
                g("Description", &[c("4", "Age", 1), c("5", "Sex", 1), c("6", "Color", 1)]),
                g("", &[c("7", "Occupation", 4)]),
                g("Value of estate", &[c("8", "Real estate", 2), c("9", "Personal estate", 2)]),
                g("", &[c("10", "Place of birth", 4)]),
                g("Parentage", &[c("11", "Father foreign born", 1), c("12", "Mother foreign born", 1)]),
                g("", &[c("13", "Month born, if within the year", 1), c("14", "Month married, if within the year", 1)]),
                g("Education", &[c("15", "At school within the year", 1), c("16", "Cannot read", 1), c("17", "Cannot write", 1)]),
                g("", &[c("18", "Deaf & dumb, blind, insane or idiotic", 2)]),
                g("Male citizens 21 & over", &[c("19", "Citizen", 1), c("20", "Right to vote denied", 1)]),
                NOTES,
            ],
        }],
        note: "The first census after the Civil War, naming formerly enslaved people for the first time.",
    },
    Form {
        country: "United States",
        year: 1880,
        day: "1 June 1880",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("In cities", &[c("", "Street", 3), c("", "House number", 1)]),
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7)]),
                g("Description", &[c("4", "Color", 1), c("5", "Sex", 1), c("6", "Age", 1), c("7", "Month born, if within the year", 1)]),
                g("", &[c("8", "Relationship to head", 2)]),
                g("Civil condition", &[c("9", "Single", 1), c("10", "Married", 1), c("11", "Widowed / divorced", 1), c("12", "Married within the year", 1)]),
                g("Occupation", &[c("13", "Occupation", 4), c("14", "Months unemployed", 1)]),
                g("Health", &[c("15", "Sick or disabled today: what", 2), c("16", "Blind", 1), c("17", "Deaf & dumb", 1), c("18", "Idiotic", 1), c("19", "Insane", 1), c("20", "Maimed, crippled, bedridden", 1)]),
                g("Education", &[c("21", "At school within the year", 1), c("22", "Cannot read", 1), c("23", "Cannot write", 1)]),
                g("Place of birth", &[c("24", "This person", 3), c("25", "Father", 3), c("26", "Mother", 3)]),
            ],
        }],
        note: "The first census to record each person's relationship to the head of the household, and parents' birthplaces.",
    },
    Form {
        country: "United States",
        year: 1890,
        day: "2 June 1890",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("", "Dwelling number", 1), c("", "Family number", 1)]),
                g("Name", &[c("1", "Given name & middle initial", 4), c("2", "Surname", 3)]),
                g("", &[c("3", "Civil War soldier, sailor, marine or widow", 1), c("4", "Relationship to head", 2), c("5", "Race", 1), c("6", "Sex", 1), c("7", "Age", 1), c("8", "Marital status", 1), c("9", "Married within the year", 1), c("10", "Mother of how many children / how many living", 2)]),
                g("Place of birth", &[c("11", "This person", 3), c("12", "Father", 3), c("13", "Mother", 3)]),
                g("Immigration", &[c("14", "Years in the U.S.", 1), c("15", "Naturalized", 1), c("16", "Naturalization papers taken out", 1)]),
                g("", &[c("17", "Occupation", 4), c("18", "Months unemployed", 1), c("19", "Months at school", 1), c("20", "Can read", 1), c("21", "Can write", 1), c("22", "Speaks English (or language)", 2)]),
                g("Health", &[c("23", "Disease and how long", 2), c("24", "Defect", 2), c("25", "Prisoner, convict, homeless child or pauper", 1)]),
            ],
        }],
        note: "Most 1890 population schedules were destroyed after a fire in 1921; fragments survive for parts of a few states. The original was a family schedule with each person in a column; here each person has a line. A separate schedule listed Union veterans and widows.",
    },
    Form {
        country: "United States",
        year: 1900,
        day: "1 June 1900",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("Location", &[c("", "Street", 3), c("", "House number", 1)]),
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7), c("4", "Relationship to head", 2)]),
                g("Description", &[c("5", "Color or race", 1), c("6", "Sex", 1), c("7", "Month & year of birth", 2), c("8", "Age", 1), c("9", "Marital status", 1), c("10", "Years married", 1), c("11", "Mother of how many children", 1), c("12", "Number living", 1)]),
                g("Place of birth", &[c("13", "This person", 3), c("14", "Father", 3), c("15", "Mother", 3)]),
                g("Citizenship", &[c("16", "Year of immigration", 1), c("17", "Years in the U.S.", 1), c("18", "Naturalized", 1)]),
                g("Occupation", &[c("19", "Occupation", 4), c("20", "Months not employed", 1)]),
                g("Education", &[c("21", "Months at school", 1), c("22", "Can read", 1), c("23", "Can write", 1), c("24", "Speaks English", 1)]),
                g("Home", &[c("25", "Owned or rented", 1), c("26", "Free or mortgaged", 1), c("27", "Farm or house", 1), c("28", "Farm schedule number", 1)]),
            ],
        }],
        note: "The only census to ask each person's month and year of birth.",
    },
    Form {
        country: "United States",
        year: 1910,
        day: "15 April 1910",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("Location", &[c("", "Street", 3), c("", "House number", 1)]),
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7), c("4", "Relationship to head", 2)]),
                g("Description", &[c("5", "Sex", 1), c("6", "Color or race", 1), c("7", "Age", 1), c("8", "Marital status", 1), c("9", "Years of present marriage", 1), c("10", "Mother of how many children", 1), c("11", "Number living", 1)]),
                g("Place of birth", &[c("12", "This person", 3), c("13", "Father", 3), c("14", "Mother", 3)]),
                g("Citizenship", &[c("15", "Year of immigration", 1), c("16", "Naturalized or alien", 1), c("17", "Speaks English, or language", 2)]),
                g("Occupation", &[c("18", "Occupation", 3), c("19", "Industry", 3), c("20", "Employer, employee or own account", 1), c("21", "Out of work 15 April 1910", 1), c("22", "Weeks out of work in 1909", 1)]),
                g("Education", &[c("23", "Can read", 1), c("24", "Can write", 1), c("25", "At school since 1 Sept 1909", 1)]),
                g("Home", &[c("26", "Owned or rented", 1), c("27", "Free or mortgaged", 1), c("28", "Farm or house", 1), c("29", "Farm schedule number", 1)]),
                g("", &[c("30", "Union or Confederate veteran", 1), c("31", "Blind (both eyes)", 1), c("32", "Deaf and dumb", 1)]),
            ],
        }],
        note: "",
    },
    Form {
        country: "United States",
        year: 1920,
        day: "1 January 1920",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("Place of abode", &[c("1", "Street", 3), c("2", "House number or farm", 1), c("3", "Dwelling number", 1), c("4", "Family number", 1)]),
                g("", &[c("5", "Name", 7), c("6", "Relationship to head", 2)]),
                g("Tenure", &[c("7", "Owned or rented", 1), c("8", "Free or mortgaged", 1)]),
                g("Description", &[c("9", "Sex", 1), c("10", "Color or race", 1), c("11", "Age", 1), c("12", "Marital status", 1)]),
                g("Citizenship", &[c("13", "Year of immigration", 1), c("14", "Naturalized or alien", 1), c("15", "Year naturalized", 1)]),
                g("Education", &[c("16", "At school since 1 Sept 1919", 1), c("17", "Can read", 1), c("18", "Can write", 1)]),
                g("Nativity and mother tongue", &[c("19", "Birthplace", 3), c("20", "Mother tongue", 2), c("21", "Father's birthplace", 3), c("22", "Father's mother tongue", 2), c("23", "Mother's birthplace", 3), c("24", "Mother's mother tongue", 2)]),
                g("", &[c("25", "Speaks English", 1)]),
                g("Occupation", &[c("26", "Occupation", 3), c("27", "Industry", 3), c("28", "Employer, wage worker or own account", 1), c("29", "Farm schedule number", 1)]),
            ],
        }],
        note: "",
    },
    Form {
        country: "United States",
        year: 1930,
        day: "1 April 1930",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("Place of abode", &[c("1", "Street", 3), c("2", "House number", 1), c("3", "Dwelling number", 1), c("4", "Family number", 1)]),
                g("", &[c("5", "Name", 7), c("6", "Relationship to head", 2)]),
                g("Home data", &[c("7", "Owned or rented", 1), c("8", "Value or monthly rent", 1), c("9", "Radio set", 1), c("10", "Lives on a farm", 1)]),
                g("Description", &[c("11", "Sex", 1), c("12", "Color or race", 1), c("13", "Age", 1), c("14", "Marital status", 1), c("15", "Age at first marriage", 1)]),
                g("Education", &[c("16", "At school since 1 Sept 1929", 1), c("17", "Can read and write", 1)]),
                g("Place of birth", &[c("18", "This person", 3), c("19", "Father", 3), c("20", "Mother", 3)]),
                g("Mother tongue and citizenship", &[c("21", "Language before coming to the U.S.", 2), c("22", "Year of immigration", 1), c("23", "Naturalization", 1), c("24", "Speaks English", 1)]),
                g("Occupation", &[c("25", "Occupation", 3), c("26", "Industry", 3), c("27", "Class of worker", 1)]),
                g("Employment", &[c("28", "At work yesterday", 1), c("29", "Unemployment schedule line", 1)]),
                g("Veterans", &[c("30", "Veteran", 1), c("31", "Which war", 1)]),
                g("", &[c("32", "Farm schedule number", 1)]),
            ],
        }],
        note: "",
    },
    Form {
        country: "United States",
        year: 1940,
        day: "1 April 1940",
        place: "",
        households: false,
        sheets: &[
            Sheet {
                title: "Everyone",
                groups: &[
                    g("Location", &[c("1", "Street", 3), c("2", "House number", 1)]),
                    g("Household", &[c("3", "Household number", 1), c("4", "Owned or rented", 1), c("5", "Value or monthly rent", 1), c("6", "On a farm", 1)]),
                    g("", &[c("7", "Name", 7), c("8", "Relationship to head", 2)]),
                    g("Description", &[c("9", "Sex", 1), c("10", "Color or race", 1), c("11", "Age", 1), c("12", "Marital status", 1)]),
                    g("Education", &[c("13", "At school since 1 Mar 1940", 1), c("14", "Highest grade completed", 1)]),
                    g("", &[c("15", "Place of birth", 3), c("16", "Citizenship (foreign born)", 1)]),
                    g("Residence on 1 April 1935", &[c("17", "City or town", 2), c("18", "County", 2), c("19", "State or country", 2), c("20", "On a farm", 1)]),
                    g("Persons 14 and over: work", &[c("21", "At work 24-30 Mar", 1), c("22", "Public emergency work", 1), c("23", "Seeking work", 1), c("24", "Had a job", 1), c("25", "If not: house, school, unable, other", 1), c("26", "Hours worked 24-30 Mar", 1), c("27", "Weeks unemployed", 1)]),
                    g("Occupation", &[c("28", "Occupation", 3), c("29", "Industry", 3), c("30", "Class of worker", 1)]),
                    g("1939", &[c("31", "Weeks worked", 1), c("32", "Wages or salary", 1), c("33", "Other income over $50", 1)]),
                    g("", &[c("34", "Farm schedule number", 1)]),
                ],
            },
            Sheet {
                title: "Supplementary questions (two people on each sheet, on lines 14 and 29)",
                groups: &[
                    g("", &[c("", "Line", 1), c("35", "Name", 6)]),
                    g("Place of birth", &[c("36", "Father", 3), c("37", "Mother", 3)]),
                    g("", &[c("38", "Mother tongue", 2)]),
                    g("Veterans", &[c("39", "Veteran, or wife, widow or child of one", 1), c("40", "If child: is the veteran father dead", 1), c("41", "War or military service", 1)]),
                    g("Social Security", &[c("42", "Has a number", 1), c("43", "Deductions in 1939", 1), c("44", "Deductions from all, half or part of wages", 1)]),
                    g("Usual work", &[c("45", "Occupation", 3), c("46", "Industry", 3), c("47", "Class of worker", 1)]),
                    g("Women ever married", &[c("48", "Married more than once", 1), c("49", "Age at first marriage", 1), c("50", "Children ever born", 1)]),
                ],
            },
        ],
        note: "Look for an X or circled initials beside a name: they mark the person who gave the information. Only the people on lines 14 and 29 of each sheet were asked the supplementary questions.",
    },
    Form {
        country: "United States",
        year: 1950,
        day: "1 April 1950",
        place: "",
        households: false,
        sheets: &[
            Sheet {
                title: "Everyone",
                groups: &[
                    g("", &[c("", "Line", 1)]),
                    g("Location", &[c("1", "Street", 3), c("2", "House or apartment number", 1), c("3", "Dwelling unit serial number", 1)]),
                    g("Farm", &[c("4", "On a farm or ranch", 1), c("5", "If not, on 3+ acres", 1), c("6", "Agriculture questionnaire number", 1)]),
                    g("", &[c("7", "Name", 7), c("8", "Relationship to head", 2)]),
                    g("Description", &[c("9", "Race", 1), c("10", "Sex", 1), c("11", "Age", 1), c("12", "Marital status", 1)]),
                    g("", &[c("13", "State or country of birth", 3), c("14", "If foreign born: naturalized", 1)]),
                    g("Persons 14 and over", &[c("15", "Doing most of last week", 1), c("16", "Any work last week", 1), c("17", "Looking for work", 1), c("18", "Has a job or business", 1), c("19", "Hours worked last week", 1)]),
                    g("Work", &[c("20a", "Occupation", 3), c("20b", "Industry", 3), c("20c", "Class of worker", 1)]),
                ],
            },
            Sheet {
                title: "Sample line questions (about one person in five)",
                groups: &[
                    g("", &[c("", "Line", 1), c("", "Name", 5)]),
                    g("A year ago", &[c("21", "Same house", 1), c("22", "On a farm", 1), c("23", "Same county", 1), c("24a", "County", 2), c("24b", "State or country", 2)]),
                    g("", &[c("25", "Country of birth of father and mother", 2)]),
                    g("School", &[c("26", "Highest grade attended", 1), c("27", "Finished it", 1), c("28", "At school since 1 Feb", 1)]),
                    g("Age 14 and over", &[c("29", "Weeks looking for work", 1), c("30", "Weeks worked in 1949", 1)]),
                    g("1949 income", &[c("31a", "Wages", 1), c("31b", "Own business", 1), c("31c", "Other", 1), c("32a", "Relatives' wages", 1), c("32b", "Relatives' business", 1), c("32c", "Relatives' other", 1)]),
                    g("Men: armed forces", &[c("33a", "World War I", 1), c("33b", "World War II", 1), c("33c", "Other times", 1)]),
                    g("Last sample line only", &[c("35a", "Last occupation", 2), c("35b", "Industry", 2), c("35c", "Class", 1), c("36", "Married more than once", 1), c("37", "Years since marriage ended", 1), c("38", "Children ever born", 1)]),
                ],
            },
        ],
        note: "The sample questions were asked of the people on the marked sample lines. Item 34 is a check box for the enumerator.",
    },
];

const CA_NAME_AGAIN: &[Col] = &[c("", "Name (as on sheet 1)", 6)];

pub const CA: &[Form] = &[
    Form {
        country: "Canada",
        year: 1851,
        day: "12 January 1852",
        place: "Canada East and Canada West",
        households: false,
        sheets: &[
            Sheet {
                title: "People",
                groups: &[
                    g("", &[c("1", "Name", 7), c("2", "Occupation", 3), c("3", "Place of birth", 3), c("4", "Religion", 2), c("5", "Residence, if outside the limits", 2), c("6", "Age at next birthday", 1)]),
                    g("Sex", &[c("7", "Male", 1), c("8", "Female", 1)]),
                    g("", &[c("9", "Married or single", 1), c("10", "Colored persons", 1), c("11", "Indians", 1)]),
                    g("Family members living here", &[c("12", "Male", 1), c("13", "Female", 1)]),
                    g("Others living here", &[c("14", "Male", 1), c("15", "Female", 1)]),
                    g("Family members away", &[c("16", "Male", 1), c("17", "Female", 1)]),
                    g("Deaf and dumb", &[c("18", "Male", 1), c("19", "Female", 1)]),
                    g("Blind", &[c("20", "Male", 1), c("21", "Female", 1)]),
                    g("Lunatics", &[c("22", "Male", 1), c("23", "Female", 1)]),
                    g("At school", &[c("24", "Male", 1), c("25", "Female", 1)]),
                    g("Births in 1851", &[c("26", "Male", 1), c("27", "Female", 1)]),
                    g("Deaths in 1851", &[c("28", "Male", 1), c("29", "Female", 1), c("30", "Age and cause", 2)]),
                ],
            },
            Sheet {
                title: "Houses and buildings",
                groups: &[
                    g("", CA_NAME_AGAIN),
                    g("House", &[c("31", "Built of", 2), c("32", "Storeys", 1), c("33", "Families in it", 1), c("34", "Vacant", 1), c("35", "Being built", 1)]),
                    g("", &[c("36", "Shops, stores, inns, taverns", 2), c("37", "Public buildings", 2), c("38", "Places of worship", 2), c("39", "Mills and factories", 2), c("40", "Persons employed", 1), c("41", "Remarks", 6)]),
                ],
            },
        ],
        note: "Called the 1851 census, but taken in January 1852. These are the rural columns for Canada West (Ontario); Canada East (Quebec) rural schedules match. Urban schedules number the columns differently from column 10 on, and New Brunswick and Nova Scotia used their own forms.",
    },
    Form {
        country: "Canada",
        year: 1861,
        day: "14 January 1861",
        place: "Canada East and Canada West",
        households: false,
        sheets: &[
            Sheet {
                title: "People",
                groups: &[
                    g("", &[c("1", "Name", 7), c("2", "Occupation", 3), c("3", "Place of birth", 3), c("4", "Married during the year", 1), c("5", "Religion", 2), c("6", "Residence, if outside the limits", 2), c("7", "Age at next birthday", 1)]),
                    g("Sex", &[c("8", "Male", 1), c("9", "Female", 1)]),
                    g("", &[c("10", "Married or single", 1), c("11", "Widowers", 1), c("12", "Widows", 1), c("13", "Colored, mulatto or Indian", 1)]),
                    g("Family members living here", &[c("14", "Male", 1), c("15", "Female", 1)]),
                    g("Others living here", &[c("16", "Male", 1), c("17", "Female", 1)]),
                    g("Family members away", &[c("18", "Male", 1), c("19", "Female", 1)]),
                    g("", &[c("20", "Deaf or dumb", 1), c("21", "Blind", 1), c("22", "Lunatic or idiot", 1)]),
                    g("At school", &[c("23", "Male", 1), c("24", "Female", 1)]),
                    g("Over 20, cannot read or write", &[c("25", "Male", 1), c("26", "Female", 1)]),
                    g("Births in 1860", &[c("27", "Male", 1), c("28", "Female", 1)]),
                    g("Deaths in 1860", &[c("29", "Male", 1), c("30", "Female", 1), c("31", "Age and cause", 2)]),
                ],
            },
            Sheet {
                title: "House, animals, carriages and business",
                groups: &[
                    g("", CA_NAME_AGAIN),
                    g("House", &[c("32", "Built of", 2), c("33", "Storeys", 1), c("34", "Families in it", 1), c("35", "Vacant", 1), c("36", "Being built", 1)]),
                    g("Animals (not farmers)", &[c("37", "Horses", 1), c("38", "Cows", 1), c("39", "Sheep", 1), c("40", "Pigs", 1), c("41", "Value $", 1)]),
                    g("Carriages", &[c("42", "Pleasure", 1), c("43", "Value $", 1), c("44", "For hire", 1), c("45", "Value $", 1)]),
                    g("", &[c("46", "Land with the house", 1)]),
                    g("Business or manufacture", &[c("47", "Name or kind", 3), c("48", "Capital invested", 1), c("49", "Raw material: quantity", 1), c("50", "Raw material: kind", 1), c("51", "Raw material: value", 1), c("52", "Power or machinery", 1), c("53", "Men employed", 1), c("54", "Women employed", 1), c("55", "Monthly cost: men", 1), c("56", "Monthly cost: women", 1), c("57", "Product: quantity", 1), c("58", "Product: kind", 1), c("59", "Product: value", 1)]),
                    g("", &[c("60", "Remarks", 4)]),
                ],
            },
        ],
        note: "These are the columns for Canada East and Canada West; the Canada East form has the same columns with French headings. The Maritime colonies used their own forms.",
    },
    Form {
        country: "Canada",
        year: 1871,
        day: "2 April 1871",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("Buildings", &[c("1", "Vessels", 1), c("2", "Shanties", 1), c("3", "Houses being built", 1), c("4", "Houses empty", 1), c("5", "Houses lived in", 1)]),
                g("", &[c("6", "Families", 1), c("7", "Name", 7), c("8", "Sex", 1), c("9", "Age", 1), c("10", "Born in the last 12 months", 1), c("11", "Country or province of birth", 3), c("12", "Religion", 2), c("13", "Origin", 2), c("14", "Occupation", 4), c("15", "Married or widowed", 1), c("16", "Married in the last 12 months", 1)]),
                g("Education", &[c("17", "At school", 1), c("18", "Over 20, cannot read", 1), c("19", "Over 20, cannot write", 1)]),
                g("Infirmities", &[c("20", "Deaf and dumb", 1), c("21", "Blind", 1), c("22", "Unsound mind", 1)]),
                g("", &[c("23", "Dates and remarks", 4)]),
            ],
        }],
        note: "Taken in Ontario, Quebec, New Brunswick and Nova Scotia. \"Origin\" is the ethnic origin traced through the father.",
    },
    Form {
        country: "Canada",
        year: 1881,
        day: "4 April 1881",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("Buildings", &[c("1", "Vessels", 1), c("2", "Shanties", 1), c("3", "Houses being built", 1), c("4", "Houses empty", 1), c("5", "Houses lived in", 1)]),
                g("", &[c("6", "Families", 1), c("7", "Name", 7), c("8", "Sex", 1), c("9", "Age", 1), c("10", "Born in the last 12 months", 1), c("11", "Country or province of birth", 3), c("12", "Religion", 2), c("13", "Origin", 2), c("14", "Occupation", 4), c("15", "Married or widowed", 1), c("16", "At school", 1)]),
                g("Infirmities", &[c("17", "Deaf and dumb", 1), c("18", "Blind", 1), c("19", "Unsound mind", 1)]),
                g("", &[c("20", "Dates and remarks", 4)]),
            ],
        }],
        note: "\"Origin\" is the ethnic origin traced through the father.",
    },
    Form {
        country: "Canada",
        year: 1891,
        day: "6 April 1891",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("Buildings", &[c("1", "Vessels and shanties", 1), c("2", "Houses being built", 1), c("3", "Houses empty", 1), c("4", "Houses lived in", 1)]),
                g("", &[c("5", "Families", 1), c("6", "Name", 7), c("7", "Sex", 1), c("8", "Age", 1), c("9", "Married or widowed", 1), c("10", "Relationship to head", 2)]),
                g("Place of birth", &[c("11", "Country or province", 3), c("12", "French Canadian", 1), c("13", "Father", 2), c("14", "Mother", 2)]),
                g("", &[c("15", "Religion", 2), c("16", "Occupation", 4)]),
                g("Work", &[c("17", "Employer", 1), c("18", "Wage earner", 1), c("19", "Out of work the week before", 1), c("20", "Hands employed (employers)", 1)]),
                g("Education", &[c("21", "Can read", 1), c("22", "Can write", 1)]),
                g("Infirmities", &[c("23", "Deaf and dumb", 1), c("24", "Blind", 1), c("25", "Unsound mind", 1)]),
            ],
        }],
        note: "",
    },
    Form {
        country: "Canada",
        year: 1901,
        day: "31 March 1901",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7), c("4", "Sex", 1), c("5", "Colour", 1), c("6", "Relationship to head", 2), c("7", "Single, married, widowed, divorced", 1)]),
                g("Birth", &[c("8", "Month and day", 1), c("9", "Year", 1), c("10", "Age at last birthday", 1), c("11", "Country or place", 3)]),
                g("", &[c("12", "Year of immigration", 1), c("13", "Year naturalized", 1), c("14", "Racial or tribal origin", 2), c("15", "Nationality", 2), c("16", "Religion", 2), c("17", "Occupation", 3)]),
                g("Work", &[c("18", "Own means", 1), c("19", "Employer", 1), c("20", "Employee", 1), c("21", "Own account", 1), c("22", "Works at trade in factory or home", 1), c("23", "Months in factory", 1), c("24", "Months at home", 1), c("25", "Months at other work", 1), c("26", "Earnings $", 1), c("27", "Extra earnings $", 1)]),
                g("Education and language", &[c("28", "Months at school", 1), c("29", "Can read", 1), c("30", "Can write", 1), c("31", "Speaks English", 1), c("32", "Speaks French", 1), c("33", "Mother tongue", 2)]),
                g("", &[c("34", "Infirmities", 1)]),
            ],
        }],
        note: "The only Canadian census to ask each person's full date of birth.",
    },
    Form {
        country: "Canada",
        year: 1906,
        day: "24 June 1906",
        place: "Manitoba, Saskatchewan and Alberta",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Family number", 1), c("2", "Name", 7), c("3", "Relationship to head", 2), c("4", "Sex", 1), c("5", "Marital status", 1), c("6", "Age", 1), c("7", "Place of birth", 3), c("8", "Year of immigration", 1), c("9", "Post office", 3)]),
                g("Land location", &[c("10", "Section", 1), c("11", "Township", 1), c("12", "Range", 1), c("13", "Meridian", 1)]),
                g("Livestock", &[c("14", "Horses", 1), c("15", "Milch cows", 1), c("16", "Other cattle", 1), c("17", "Sheep", 1), c("18", "Pigs", 1)]),
            ],
        }],
        note: "A special census of the three Prairie provinces only.",
    },
    Form {
        country: "Canada",
        year: 1911,
        day: "1 June 1911",
        place: "",
        households: false,
        sheets: &[
            Sheet {
                title: "People",
                groups: &[
                    g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7), c("4", "Place of habitation", 3), c("5", "Sex", 1), c("6", "Relationship to head", 2), c("7", "Marital status", 1)]),
                    g("Birth", &[c("8", "Month", 1), c("9", "Year", 1), c("10", "Age at last birthday", 1), c("11", "Country or place", 3)]),
                    g("", &[c("12", "Year of immigration", 1), c("13", "Year naturalized", 1), c("14", "Racial or tribal origin", 2), c("15", "Nationality", 2), c("16", "Religion", 2)]),
                    g("Occupation", &[c("17", "Chief occupation", 3), c("18", "Other work", 2), c("19", "Employer", 1), c("20", "Employee", 1), c("21", "Own account", 1), c("22", "Where employed", 3)]),
                ],
            },
            Sheet {
                title: "Work, insurance, education and infirmities",
                groups: &[
                    g("", CA_NAME_AGAIN),
                    g("Weeks worked in 1910", &[c("23", "Chief occupation", 1), c("24", "Other work", 1)]),
                    g("Hours a week", &[c("25", "Chief occupation", 1), c("26", "Other work", 1)]),
                    g("Earnings in 1910", &[c("27", "Chief occupation $", 1), c("28", "Other work $", 1), c("29", "Rate per hour", 1)]),
                    g("Insurance", &[c("30", "Life", 1), c("31", "Accident or sickness", 1), c("32", "Cost in the year", 1)]),
                    g("Education", &[c("33", "Months at school in 1910", 1), c("34", "Can read", 1), c("35", "Can write", 1), c("36", "Language usually spoken", 2), c("37", "Cost of education (over 16)", 1)]),
                    g("Infirmities", &[c("38", "Blind", 1), c("39", "Deaf and dumb", 1), c("40", "Insane", 1), c("41", "Idiotic", 1)]),
                ],
            },
        ],
        note: "Columns 38 to 41 were lost from most of the microfilm, so they are usually missing from the images.",
    },
    Form {
        country: "Canada",
        year: 1916,
        day: "1 June 1916",
        place: "Manitoba, Saskatchewan and Alberta",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Dwelling number", 1), c("2", "Family, household or institution", 1), c("3", "Name", 7), c("4", "Military service", 1)]),
                g("Place of abode", &[c("5", "Township", 1), c("6", "Range", 1), c("7", "Meridian", 1), c("8", "Municipality", 2)]),
                g("", &[c("9", "Relationship to head", 2), c("10", "Sex", 1), c("11", "Marital status", 1), c("12", "Age at last birthday", 1), c("13", "Country or place of birth", 3), c("14", "Religion", 2), c("15", "Year of immigration", 1), c("16", "Year naturalized", 1), c("17", "Nationality", 2), c("18", "Racial or tribal origin", 2)]),
                g("Language and education", &[c("19", "Speaks English", 1), c("20", "Speaks French", 1), c("21", "Mother tongue", 2), c("22", "Can read", 1), c("23", "Can write", 1)]),
                g("Occupation", &[c("24", "Chief occupation", 3), c("25", "Employer, employee or own account", 1), c("26", "Where employed", 3)]),
            ],
        }],
        note: "A special census of the three Prairie provinces only. Column 4 records military service during the First World War.",
    },
    Form {
        country: "Canada",
        year: 1921,
        day: "1 June 1921",
        place: "",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7)]),
                g("Place of abode", &[c("4", "Rural (section, township, range)", 2), c("5", "Urban (street and number)", 2)]),
                g("Home", &[c("6", "Owned or rented", 1), c("7", "Monthly rent", 1), c("8", "Class of house", 1), c("9", "Built of", 1), c("10", "Rooms", 1)]),
                g("", &[c("11", "Relationship to head", 2), c("12", "Sex", 1), c("13", "Marital status", 1), c("14", "Age", 1)]),
                g("Place of birth", &[c("15", "This person", 2), c("16", "Father", 2), c("17", "Mother", 2)]),
                g("", &[c("18", "Year of immigration", 1), c("19", "Year naturalized", 1), c("20", "Nationality", 1), c("21", "Racial or tribal origin", 1)]),
                g("Language, religion, education", &[c("22", "Speaks English", 1), c("23", "Speaks French", 1), c("24", "Mother tongue", 1), c("25", "Religion", 1), c("26", "Can read", 1), c("27", "Can write", 1), c("28", "Months at school", 1)]),
                g("Occupation", &[c("29", "Chief occupation", 3), c("30", "Employer, employee or own account", 1), c("31", "Where employed", 3), c("32", "Earnings in last 12 months", 1), c("33", "Out of work 1 June 1921", 1), c("34", "Weeks out of work", 1), c("35", "Weeks out of work from illness", 1)]),
            ],
        }],
        note: "",
    },
    Form {
        country: "Canada",
        year: 1931,
        day: "1 June 1931",
        place: "",
        households: false,
        sheets: &[
            Sheet {
                title: "People",
                groups: &[
                    g("", &[c("1", "Dwelling number", 1), c("2", "Family number", 1), c("3", "Name", 7), c("4", "Place of abode", 3)]),
                    g("Home", &[c("5", "Owned or rented", 1), c("6", "Value or monthly rent", 1), c("7", "Class of house", 1), c("8", "Built of", 1), c("9", "Rooms", 1), c("10", "Radio", 1)]),
                    g("", &[c("11", "Relationship to head", 2), c("12", "Sex", 1), c("13", "Marital status", 1), c("14", "Age at last birthday", 1)]),
                    g("Place of birth", &[c("15", "This person", 2), c("16", "Father", 2), c("17", "Mother", 2)]),
                    g("", &[c("18", "Year of immigration", 1), c("19", "Year naturalized", 1), c("20", "Nationality", 1), c("21", "Racial origin", 1)]),
                    g("Language, religion, education", &[c("22", "Speaks English", 1), c("23", "Speaks French", 1), c("24", "Mother tongue", 1), c("25", "Religion", 2), c("26", "Can read and write", 1), c("27", "Months at school", 1)]),
                ],
            },
            Sheet {
                title: "Work and unemployment",
                groups: &[
                    g("", CA_NAME_AGAIN),
                    g("Occupation", &[c("28", "Chief occupation", 3), c("29", "Industry", 3), c("30", "Class of worker", 1), c("31", "Total earnings in the year", 1)]),
                    g("On 1 June 1931", &[c("32", "At work", 1), c("33", "If not, why", 2)]),
                    g("Weeks out of work in the year", &[c("34", "Total", 1), c("35", "No job", 1), c("36", "Illness", 1), c("37", "Accident", 1), c("38", "Strike or lockout", 1), c("39", "Temporary layoff", 1), c("40", "Other reasons", 1)]),
                ],
            },
        ],
        note: "The first Canadian census to ask about a radio. The Northwest Territories used a shorter form.",
    },
];

const UK_ADDRESS: &[Col] = &[c("", "Schedule number", 1), c("", "Address (street, house name or number)", 4)];
const UK_HOUSES: Group = g("Houses", &[c("", "Inhabited", 1), c("", "Uninhabited (U) or building (B)", 1)]);
const UK_PERSON: &[Col] = &[c("", "Name and surname", 7), c("", "Relation to head", 2), c("", "Condition (single, married, widow)", 1)];
const UK_AGE: Group = g("Age", &[c("", "Males", 1), c("", "Females", 1)]);

pub const UK: &[Form] = &[
    Form {
        country: "United Kingdom",
        year: 1841,
        day: "6 June 1841",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("", "Place (street, house name or number)", 4)]),
                g("Houses", &[c("", "Uninhabited or building", 1), c("", "Inhabited", 1)]),
                g("", &[c("", "Name and surname", 7)]),
                UK_AGE,
                g("", &[c("", "Profession, trade, employment or independent means", 4)]),
                g("Where born", &[c("", "In the same county (yes / no)", 1), c("", "Scotland (S), Ireland (I) or foreign parts (F)", 1)]),
                NOTES,
            ],
        }],
        note: "No relationships are given. Ages over 15 were usually rounded down to a multiple of 5 (a person of 37 shows as 35), and birthplace only says whether someone was born in the same county.",
    },
    Form {
        country: "United Kingdom",
        year: 1851,
        day: "30 March 1851",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[g("", UK_ADDRESS), g("", UK_PERSON), UK_AGE, g("", &[c("", "Rank, profession or occupation", 4), c("", "Where born (county and parish, or country)", 4), c("", "Deaf and dumb or blind", 1)]), NOTES],
        }],
        note: "The first census to give each person's relation to the head, marital condition, exact age and parish of birth. Scotland's 1851 census asked much the same.",
    },
    Form {
        country: "United Kingdom",
        year: 1861,
        day: "7 April 1861",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[g("", UK_ADDRESS), UK_HOUSES, g("", UK_PERSON), UK_AGE, g("", &[c("", "Rank, profession or occupation", 4), c("", "Where born", 4), c("", "Deaf and dumb, blind, imbecile or lunatic", 1)]), NOTES],
        }],
        note: "Scotland also recorded the number of rooms with one or more windows, and children 5 to 13 at school.",
    },
    Form {
        country: "United Kingdom",
        year: 1871,
        day: "2 April 1871",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[g("", UK_ADDRESS), UK_HOUSES, g("", UK_PERSON), UK_AGE, g("", &[c("", "Rank, profession or occupation", 4), c("", "Where born", 4), c("", "Deaf and dumb, blind, imbecile or idiot, lunatic", 1)]), NOTES],
        }],
        note: "Scotland also recorded rooms with one or more windows, and children 5 to 13 at school.",
    },
    Form {
        country: "United Kingdom",
        year: 1881,
        day: "3 April 1881",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[g("", UK_ADDRESS), UK_HOUSES, g("", UK_PERSON), UK_AGE, g("", &[c("", "Rank, profession or occupation", 4), c("", "Where born", 4), c("", "Deaf and dumb, blind, imbecile or idiot, lunatic", 1)]), NOTES],
        }],
        note: "Scotland also recorded rooms with one or more windows.",
    },
    Form {
        country: "United Kingdom",
        year: 1891,
        day: "5 April 1891",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Schedule number", 1), c("2", "Address (street, house name or number)", 4)]),
                g("Houses", &[c("3", "Inhabited", 1), c("4", "Uninhabited (U) or building (B)", 1)]),
                g("", &[c("5", "Rooms occupied, if fewer than 5", 1)]),
                g("", &[c("6", "Name and surname", 7), c("7", "Relation to head", 2), c("8", "Condition (single, married, widow)", 1)]),
                g("Age", &[c("9", "Males", 1), c("10", "Females", 1)]),
                g("Occupation", &[c("11", "Profession or occupation", 4), c("12", "Employer", 1), c("13", "Employed", 1), c("14", "Neither", 1)]),
                g("", &[c("15", "Where born", 4), c("16", "Deaf and dumb, blind, lunatic, imbecile or idiot", 1), c("", "Language spoken (Wales)", 1)]),
            ],
        }],
        note: "In Wales the language column records English, Welsh or both. Scotland also recorded rooms with windows and whether each person spoke Gaelic.",
    },
    Form {
        country: "United Kingdom",
        year: 1901,
        day: "31 March 1901",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", UK_ADDRESS),
                g("Houses", &[c("", "In occupation", 1), c("", "Not in occupation", 1), c("", "Building", 1)]),
                g("", &[c("", "Rooms occupied, if fewer than 5", 1)]),
                g("", UK_PERSON),
                UK_AGE,
                g("Occupation", &[c("", "Profession or occupation", 4), c("", "Employer, worker or own account", 1), c("", "Working at home", 1)]),
                g("", &[c("", "Where born", 4), c("", "Deaf and dumb, blind, lunatic, imbecile or feeble-minded", 1), c("", "Language spoken (Wales)", 1)]),
            ],
        }],
        note: "Scotland also recorded rooms with windows and Gaelic speakers.",
    },
    Form {
        country: "United Kingdom",
        year: 1911,
        day: "2 April 1911",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("1", "Name and surname", 7), c("2", "Relationship to head", 2)]),
                g("Age", &[c("3", "Males", 1), c("4", "Females", 1)]),
                g("Marriage", &[c("5", "Single, married, widower or widow", 1), c("6", "Years of present marriage", 1)]),
                g("Children born alive to present marriage", &[c("7", "Total", 1), c("8", "Still living", 1), c("9", "Have died", 1)]),
                g("Occupation (age 10 and over)", &[c("10", "Personal occupation", 4), c("11", "Industry or service", 3), c("12", "Employer, worker or own account", 1), c("13", "Working at home", 1)]),
                g("", &[c("14", "Birthplace", 4), c("15", "Nationality (if born abroad)", 2), c("16", "Infirmity", 1), c("", "Language (Wales)", 1)]),
            ],
        }],
        note: "Filled in by the householder on a household schedule, so the handwriting is usually the family's own. Write the schedule's number of rooms (counting the kitchen) in the boxes at the top. Scotland also asked about Gaelic.",
    },
    Form {
        country: "United Kingdom",
        year: 1921,
        day: "19 June 1921",
        place: "England and Wales",
        households: false,
        sheets: &[Sheet {
            title: "",
            groups: &[
                g("", &[c("", "Name and surname", 7), c("", "Relationship to head", 2)]),
                g("Age", &[c("", "Years", 1), c("", "Months", 1)]),
                g("", &[c("", "Sex", 1)]),
                g("Marriage or orphanhood", &[c("", "Age 15+: single, married, widowed or divorced", 1), c("", "Under 15: both parents alive, father dead, mother dead, both dead", 1)]),
                g("", &[c("", "Birthplace", 4), c("", "Nationality (if born abroad)", 2)]),
                g("Education", &[c("", "Whole time or part time", 1)]),
                g("Employment (age 12 and over)", &[c("", "Personal occupation", 4), c("", "Employer's name and business", 3), c("", "Place of work", 3)]),
                g("", &[c("", "Language (Wales)", 1)]),
            ],
        }],
        note: "Taken in June after a delay. Married men, widowers and widows also listed their children under 16, with ages, at the foot of the form. Scotland asked much the same.",
    },
];

impl Form {
    pub fn columns(&self) -> usize {
        self.sheets.iter().map(|s| s.groups.iter().map(|g| g.cols.len()).sum::<usize>()).sum()
    }

    /// The boxes across the top of each sheet for where the entry was found.
    fn header_fields(&self) -> Vec<&'static str> {
        if self.country == "United Kingdom" {
            // the National Archives series each year's returns are filed under
            let piece = match self.year {
                1841 | 1851 => "Piece (HO107/...)",
                1861 => "Piece (RG9/...)",
                1871 => "Piece (RG10/...)",
                1881 => "Piece (RG11/...)",
                1891 => "Piece (RG12/...)",
                1901 => "Piece (RG13/...)",
                1911 => "Piece (RG14/...)",
                _ => "Piece (RG15/...)",
            };
            return match self.year {
                ..=1901 => vec!["County", "Civil parish", "Town or village", "Enumeration district", piece, "Folio", "Page"],
                _ => vec!["County", "Registration district", "Enumeration district", piece, "Schedule number", "Address", "Rooms"],
            };
        }
        if self.country == "Canada" {
            return match self.year {
                ..=1861 => vec!["Province", "County", "Township, town or parish", "Page", "Enumerator"],
                1906 | 1916 => vec!["Province", "District (name, no.)", "Sub-district (name, no.)", "Township / range / meridian", "Page", "Enumerated on", "Enumerator"],
                _ => vec!["Province", "District (name, no.)", "Sub-district (name, no.)", "Division", "Page", "Enumerated on", "Enumerator"],
            };
        }
        match self.year {
            ..=1840 => vec!["State", "County", "Town, township or city", "Page"],
            1850..=1870 => vec!["State", "County", "Town or township", "Post office", "Page", "Enumerated on", "Enumerator"],
            1880..=1940 => vec!["State", "County", "Township or city", "Ward", "Supervisor's district", "Enumeration district", "Sheet", "Enumerated on", "Enumerator"],
            _ => vec!["State", "County", "City or place", "Enumeration district", "Sheet", "Enumerated on", "Enumerator"],
        }
    }

    pub fn title(&self) -> String {
        if self.place.is_empty() {
            format!("{} census of {}", self.year, self.country)
        } else {
            format!("{} census of {} ({})", self.year, self.country, self.place)
        }
    }

    /// The whole printable page: one table per sheet, with blank lines to write on.
    pub fn html(&self, lines: usize) -> String {
        let mut body = String::new();
        let n = self.sheets.len();
        for (i, sh) in self.sheets.iter().enumerate() {
            body.push_str("<section class=\"cs\">");
            let part = if n > 1 { format!(" &middot; sheet {} of {}{}", i + 1, n, if sh.title.is_empty() { String::new() } else { format!(": {}", esc(sh.title)) }) } else if !sh.title.is_empty() { format!(" &middot; {}", esc(sh.title)) } else { String::new() };
            let place = if self.place.is_empty() { String::new() } else { format!(" ({})", esc(self.place)) };
            body.push_str(&format!("<div class=\"ct\"><b>{} {} census</b>{}{} <span class=\"cd\">Census day: {}</span></div>", self.year, esc(self.country), place, part, self.day));
            body.push_str("<div class=\"hf\">");
            for f in self.header_fields() {
                body.push_str(&format!("<div><span>{}</span></div>", esc(f)));
            }
            body.push_str("</div><div class=\"hf\"><div><span>Where I found it (archive, film or roll, website, image number)</span></div><div class=\"by\"><span>Copied by, on</span></div></div>");
            let cols: Vec<&Col> = sh.groups.iter().flat_map(|g| g.cols.iter()).collect();
            let total: u32 = cols.iter().map(|c| c.w as u32).sum::<u32>() + 1;
            body.push_str("<table class=\"cf\"><colgroup><col style=\"width:");
            body.push_str(&format!("{:.2}%\">", 100.0 / total as f32));
            for col in &cols {
                body.push_str(&format!("<col style=\"width:{:.2}%\">", col.w as f32 * 100.0 / total as f32));
            }
            body.push_str("</colgroup><thead>");
            // Top row: group labels over their columns; ungrouped columns span both rows.
            let grouped = sh.groups.iter().any(|g| !g.label.is_empty());
            body.push_str("<tr><th class=\"ln\" rowspan=\"");
            body.push_str(if grouped { "2" } else { "1" });
            body.push_str("\">#</th>");
            let mut second = String::new();
            for gr in sh.groups {
                if gr.label.is_empty() || !grouped {
                    for col in gr.cols {
                        body.push_str(&head_cell(col, if grouped { 2 } else { 1 }));
                    }
                } else {
                    body.push_str(&format!("<th class=\"gl\" colspan=\"{}\">{}</th>", gr.cols.len(), esc(gr.label)));
                    for col in gr.cols {
                        second.push_str(&head_cell(col, 1));
                    }
                }
            }
            body.push_str("</tr>");
            if grouped {
                body.push_str(&format!("<tr>{}</tr>", second));
            }
            body.push_str("</thead><tbody>");
            for l in 1..=lines {
                body.push_str(&format!("<tr><td class=\"ln\">{}</td>", l));
                for _ in &cols {
                    body.push_str("<td></td>");
                }
                body.push_str("</tr>");
            }
            body.push_str("</tbody></table>");
            if !self.note.is_empty() && i + 1 == n {
                body.push_str(&format!("<p class=\"cn\">{}</p>", esc(self.note)));
            }
            body.push_str("</section>");
        }
        let css = "body{padding:0 4px}\
            .cs{break-after:page;page-break-after:always;break-inside:avoid}\
            .cs:last-of-type{break-after:auto;page-break-after:auto}\
            .ct{font-size:14px;margin:0 0 4px}.ct .cd{float:right;font-size:11px;color:#555}\
            .hf{display:flex;border:1px solid #b9b2a0;border-bottom:0}.hf:last-of-type{border-bottom:1px solid #b9b2a0}\
            .hf+.hf{border-top:0}.cs .hf:nth-of-type(2){margin-bottom:4px;border-bottom:1px solid #b9b2a0}\
            .hf div{flex:1;height:26px;padding:1px 4px;border-left:1px solid #b9b2a0}.hf div:first-child{border-left:0}\
            .hf div.by{flex:0 0 26%}\
            .hf span{font-size:7.5px;color:#6b5a2e;text-transform:uppercase;letter-spacing:.3px}\
            table.cf th,table.cf td{padding:1px 2px;font-size:8.5px;line-height:1.15}\
            table.cf th{text-align:center;vertical-align:bottom;font-weight:normal}\
            table.cf th.gl{font-weight:bold;vertical-align:middle;font-size:8.5px}\
            table.cf th .no{display:block;font-weight:bold;color:#6b5a2e}\
            table.cf th.v{height:92px;padding:2px 0}\
            table.cf th.v .vw{display:flex;flex-direction:column;align-items:center;justify-content:flex-end;height:100%}\
            table.cf th.v .t{writing-mode:vertical-rl;transform:rotate(180deg);display:inline-block;max-height:92px;text-align:left;white-space:normal}\
            table.cf td{height:19px}\
            table.cf .ln{text-align:center;color:#888;font-size:8px}\
            table.cf tbody tr:nth-child(5n) td{border-bottom:1.5px solid #8a8169}\
            .cn{font-size:9.5px;color:#444;margin:4px 0 0}";
        let body = format!("{}{}", LANDSCAPE_TIP, body);
        page_with_css(&self.title(), &body, true, css, false)
    }
}

/// A heading cell: narrow columns turn the words sideways so they fit.
fn head_cell(col: &Col, rowspan: u8) -> String {
    let no = if col.n.is_empty() { String::new() } else { format!("<span class=\"no\">{}</span>", col.n) };
    let rs = if rowspan > 1 { format!(" rowspan=\"{}\"", rowspan) } else { String::new() };
    if col.w <= 1 || (col.w == 2 && col.head.len() > 18) {
        format!("<th class=\"v\"{}><div class=\"vw\">{}<span class=\"t\">{}</span></div></th>", rs, no, esc(col.head))
    } else {
        format!("<th{}>{}{}</th>", rs, no, esc(col.head))
    }
}

/// Shown on screen (not printed) above the wide forms.
pub const LANDSCAPE_TIP: &str = "<div class=\"noprint\" style=\"font-size:13px;color:#555;margin:0 0 8px\">This form is wide: in the print window, choose <b>Landscape</b> (sideways) if it isn't chosen already. If it still doesn't fit, the page is shrunk to fit the paper.</div>";

pub fn forms(country: &str) -> &'static [Form] {
    match country {
        "United States" => US,
        "Canada" => CA,
        "United Kingdom" => UK,
        _ => &[],
    }
}

pub const COUNTRIES: &[(&str, bool)] = &[("United States", true), ("Canada", true), ("United Kingdom", true)];

#[derive(Default)]
pub struct CensusUi {
    pub open: bool,
    pub country: usize,
    pub year: Option<i32>,
    pub lines: usize,
}

impl App {
    pub(crate) fn census_window(&mut self, ctx: &egui::Context) {
        if !self.x.census.open {
            return;
        }
        let mut open = true;
        let mut print: Option<String> = None;
        egui::Window::new("Blank census forms").open(&mut open).resizable(false).default_width(520.0).show(ctx, |ui| {
            let s = &mut self.x.census;
            if s.lines == 0 {
                s.lines = 20;
            }
            ui.label("Print a blank worksheet to copy a census entry onto, with that year's questions as the column headings.");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Country");
                for (i, (name, ready)) in COUNTRIES.iter().enumerate() {
                    let label = if *ready { name.to_string() } else { format!("{} (coming)", name) };
                    if ui.add_enabled(*ready, egui::SelectableLabel::new(s.country == i, label)).clicked() {
                        s.country = i;
                        s.year = None;
                    }
                }
            });
            ui.separator();
            let list = forms(COUNTRIES[s.country].0);
            ui.label("Census year");
            ui.horizontal_wrapped(|ui| {
                for f in list {
                    let label = if f.country == "Canada" && (f.year == 1906 || f.year == 1916) { format!("{} Prairies", f.year) } else { f.year.to_string() };
                    if ui.selectable_label(s.year == Some(f.year), label).clicked() {
                        s.year = Some(f.year);
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Lines on each sheet");
                ui.add(egui::Slider::new(&mut s.lines, 10..=24));
            });
            if let Some(f) = s.year.and_then(|y| list.iter().find(|f| f.year == y)) {
                ui.separator();
                ui.label(RichText::new(f.title()).strong());
                ui.label(format!(
                    "Census day {}. {} columns, one line per {}{}.",
                    f.day,
                    f.columns(),
                    if f.households { "household" } else { "person" },
                    if f.sheets.len() > 1 { format!(", on {} sheets", f.sheets.len()) } else { String::new() }
                ));
                if !f.note.is_empty() {
                    ui.label(RichText::new(f.note).small());
                }
                ui.add_space(4.0);
                if ui.button("Print or save as PDF...").clicked() {
                    print = Some(f.html(s.lines));
                }
                ui.label(RichText::new("Prints on Letter paper, sideways (landscape).").small().weak());
            } else {
                ui.label(RichText::new("Choose a year.").italics().weak());
            }
        });
        if let Some(h) = print {
            self.open_print(h);
        }
        if !open {
            self.x.census.open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uk_1891_numbered_as_the_schedule() {
        let f = UK.iter().find(|f| f.year == 1891).unwrap();
        let nums: Vec<&str> = f.sheets[0].groups.iter().flat_map(|g| g.cols.iter()).map(|c| c.n).filter(|n| !n.is_empty()).collect();
        assert_eq!(nums.len(), 16);
        assert!(f.html(10).contains("RG12"));
    }

    #[test]
    fn every_uk_census_year_is_there_once() {
        let years: Vec<i32> = UK.iter().map(|f| f.year).collect();
        assert_eq!(years, (1841..=1921).step_by(10).collect::<Vec<_>>());
        // 1911's household schedule is numbered 1 to 16; the Welsh language column is extra.
        let f = UK.iter().find(|f| f.year == 1911).unwrap();
        let numbered = f.sheets[0].groups.iter().flat_map(|g| g.cols.iter()).filter(|c| !c.n.is_empty()).count();
        assert_eq!(numbered, 16);
    }

    #[test]
    fn every_canadian_census_year_is_there_once() {
        let years: Vec<i32> = CA.iter().map(|f| f.year).collect();
        assert_eq!(years, vec![1851, 1861, 1871, 1881, 1891, 1901, 1906, 1911, 1916, 1921, 1931]);
        let count = |y: i32| CA.iter().find(|f| f.year == y).unwrap().columns();
        assert_eq!((count(1871), count(1881), count(1891), count(1901), count(1906), count(1916), count(1921)), (23, 20, 25, 34, 18, 26, 35));
        // Later sheets repeat the name (unnumbered), so they hold one extra column.
        assert_eq!((count(1851), count(1861), count(1911), count(1931)), (42, 61, 42, 41));
    }

    #[test]
    fn every_us_census_year_is_there_once() {
        let years: Vec<i32> = US.iter().map(|f| f.year).collect();
        assert_eq!(years, (1790..=1950).step_by(10).collect::<Vec<_>>());
    }

    #[test]
    fn numbered_columns_run_in_order_without_gaps() {
        // Each form's numbered columns go 1, 2, 3... (with 20a, 20b... counted as one number),
        // and a later sheet carries on from the earlier one, except 1950's sample sheet, which
        // starts at 21 after the main sheet's 20c, and item 34, which is only a check box.
        for f in US.iter().chain(CA.iter()).chain(UK.iter()) {
            let mut last = 0u32;
            for sh in f.sheets {
                for col in sh.groups.iter().flat_map(|g| g.cols.iter()) {
                    if col.n.is_empty() {
                        continue;
                    }
                    let num: u32 = col.n.trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse().unwrap();
                    let lettered = col.n.ends_with(|c: char| c.is_ascii_alphabetic());
                    let ok = num == last + 1 || (lettered && num == last) || (f.year == 1950 && num == 35 && last == 33);
                    assert!(ok, "{}: column {} follows {}", f.year, col.n, last);
                    last = num;
                }
            }
        }
    }

    #[test]
    fn column_counts_match_the_schedules() {
        let numbered = |y: i32| -> usize { US.iter().find(|f| f.year == y).unwrap().sheets.iter().flat_map(|s| s.groups.iter()).flat_map(|g| g.cols.iter()).filter(|c| !c.n.is_empty()).count() };
        assert_eq!(numbered(1850), 13);
        assert_eq!(numbered(1860), 14);
        assert_eq!(numbered(1870), 20);
        assert_eq!(numbered(1880), 26);
        assert_eq!(numbered(1890), 25);
        assert_eq!(numbered(1900), 28);
        assert_eq!(numbered(1910), 32);
        assert_eq!(numbered(1920), 29);
        assert_eq!(numbered(1930), 32);
        assert_eq!(numbered(1940), 50);
    }

    #[test]
    fn early_household_brackets() {
        let f = US.iter().find(|f| f.year == 1830).unwrap();
        assert_eq!(f.sheets[0].groups[1].cols.len(), 13);
        assert_eq!(f.sheets[1].groups[1].cols.len(), 6);
        let f = US.iter().find(|f| f.year == 1820).unwrap();
        assert_eq!(f.columns(), 34);
    }

    #[test]
    fn printable_page_has_lines_and_headings() {
        for f in US.iter().chain(CA.iter()).chain(UK.iter()) {
            let h = f.html(20);
            assert!(h.contains(&format!("{} {} census", f.year, f.country)), "{}", f.year);
            assert_eq!(h.matches("<td class=\"ln\">20</td>").count(), f.sheets.len(), "{}", f.year);
            assert!(!h.contains("<th></th>"));
        }
        let h = US.iter().find(|f| f.year == 1940).unwrap().html(10);
        assert!(h.contains("Residence on 1 April 1935") && h.contains("sheet 2 of 2"));
    }

    /// Writes every form to the print folder of the temporary test directory so they can be
    /// looked at in a browser: cargo test census_pages_for_a_look -- --ignored
    #[test]
    #[ignore]
    fn census_pages_for_a_look() {
        let dir = std::env::temp_dir().join("gr-census");
        std::fs::create_dir_all(&dir).unwrap();
        for f in US.iter().chain(CA.iter()).chain(UK.iter()) {
            std::fs::write(dir.join(format!("{}-{}.html", match f.country { "Canada" => "ca", "United Kingdom" => "uk", _ => "us" }, f.year)), f.html(24)).unwrap();
        }
    }
}
