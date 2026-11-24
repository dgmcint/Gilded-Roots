# Translations

Gilded Roots is written in English, and each piece of English text is also the key used to find
its translation. Anything not translated yet simply stays in English.

- `en.json` lists every English text in the program, with where it is used (file:line) to help
  with its meaning. It is made by `python3 tools/lang_extract.py`; run that again after changing
  the program's text. It also shows how much of each language is done.
- Each language has its own file (`de.json`, `ja.json`, ...) with the same English texts and
  their translations: `"Add person": "Person hinzufügen"`. Texts can be left out or left empty.
- `python3 tools/lang_extract.py missing de` lists the German texts not translated yet.

Some texts have parts filled in when the program runs, written `{}`, `{n}` or `{:.1}`. Keep every
one of these exactly as it is, but put them wherever the language needs them:

    "Pedigree chart of {}": "Ahnentafel von {}"
    "{} of {}": "{} von {}"

If the parts of a translation do not match the English, the program shows the English instead.

Census forms stay in English on purpose: they copy the real US, Canada and UK forms.

The language is chosen in View > Language. Japanese, Chinese and Korean use the fonts that come
with the computer (Hiragino/Heiti/Apple SD Gothic on a Mac, Yu Gothic/YaHei/Malgun Gothic on
Windows, Noto Sans CJK on Linux).
