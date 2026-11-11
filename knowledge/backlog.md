# Backlog

## Fait

- [x] Bus Linap décodé : format, dialogue, consigne, Boost, arrêt (`linap.md`).
- [x] La carte de puissance obéit à un ESP qui joue la carte de commande (consigne, Boost, arrêt).
- [x] La carte de commande répond à un ESP qui joue la carte de puissance : tout ce qui est fait aux touches est lisible.
- [x] La carte de commande ne se laisse pas piloter par le bus (adresses et écritures essayées) : l'écran ne suivra pas HA.
- [x] Capacité du 5 V de la nappe : ≈ 160 mA en tout, lâche vers 180 mA. Un D1 mini y tient avec le boîtier de commande (démarrages et Boost validés).
- [x] Firmware MITM en ESPHome sur D1 mini, composant C++ (`firmware/`) : état propre, deux émulateurs autonomes, état en flash, Boost de HA borné ; 27 tests de la logique sur le Mac.
- [x] Schéma de la carte MITM, révision 3 pour la plaque d'essai (`hardware/`) : D1 mini, deux voies (pont diviseur 100k/150k, NPN avec rappel de base), rappel de DATA côté carte de puissance, C1 de 470 µF.
- [x] Plaque d'essai validée en boucle au banc (J1 relié à J2 : 100 % des appels répondus des deux côtés), puis sur le radiateur (12/11/2026) : les touches et HA pilotent la carte de puissance.

## Carte MITM

- [ ] Alimentation par la nappe avec la carte MITM : D1 et boîtier de commande sur le 5 V de la carte de puissance, démarrages du radiateur et Boost compris (les essais ont tourné sur une alimentation de labo).
- [ ] Choisir les connecteurs de la nappe (J1, J2), puis le circuit imprimé : empreintes, isolement, DRC, export (`CLAUDE.md`).
- [ ] Mesurer la place libre dans le boîtier de commande et choisir où loger la carte et comment couper la nappe (rallonge à connecteurs d'origine si possible).
- [ ] Mesurer la marge de réponse de la carte de puissance (« Reply gap » à 2, 5, 10, 20 bits, Boost à chaque palier).
- [ ] Boîtier (`enclosure/`), séparation secteur / basse tension.

## Firmware (`firmware/`)

- [ ] Ne publier « Mode » et « Setpoint » qu'à leur changement (aujourd'hui chaque seconde).
- [ ] Exposer la température à HA une fois sa trame trouvée.

## À comprendre

- [ ] Température : capture de nuit du journal des trames (« Frame log ») lancée le 12/11/2026, à comparer avec le thermomètre de la salle de bain. Candidat : le 3e octet de `75`, qui varie à chaque cycle (`AD` à `B0`, soit 45 à 48 sans le bit de parité).
- [ ] Ce que fait la carte de puissance après un long silence de la carte de commande (au-delà de 30 s).
- [ ] Sens de la réponse `70` (`83 80 89`, constante) et de `76`.

## Côté Home Assistant

- [ ] Ajouter l'appareil `sauter-marapi` (intégration ESPHome) et repointer l'automatisation et le script Boost du sèche-serviettes vers ses entités (projet `home-assistant`).
- [ ] Mesure de puissance éventuelle : Shelly PM Mini Gen3 dans la boîte de raccordement (≈ 1750 W = Boost, ≈ 750 W = fluide, 0 = attente), utile si `C7` ne suffit pas.

## Référence : carte de commande

Photographiée le 25/10/2026 ([dans son boîtier](photos/command-board-in-box.jpg), [touches et écran](photos/command-board-keys.jpg)) : étiquette `HM011 TD1N01 RAD`, circuit imprimé `74 60 5004 IndB`, microcontrôleur **STM32F091CCT6** (contrôleur tactile intégré), écran LCD Varitronix, touches capacitives (ressorts dorés vers la vitre), 5 pastilles alignées (probablement SWD, lecture du firmware sûrement verrouillée). La garantie (2 ans) est perdue dès qu'on ouvre le boîtier.
