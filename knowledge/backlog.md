# Backlog

## Fait

- [x] Bus Linap décodé : format, dialogue, consigne, Boost, arrêt (`linap.md`).
- [x] La carte de puissance obéit à un ESP qui joue la carte de commande (consigne, Boost, arrêt).
- [x] La carte de commande répond à un ESP qui joue la carte de puissance : tout ce qui est fait aux touches est lisible.
- [x] La carte de commande ne se laisse pas piloter par le bus (adresses et écritures essayées) : l'écran ne suivra pas HA.

## Carte MITM

- [ ] Redessiner `hardware/linap/` pour le MITM :
  - deux voies isolées, côté commande et côté puissance, chacune avec sa réception (H11L1M) et son émission (LTV-817) ;
  - un relais à contact repos qui relie DATA directement quand l'ESP n'est pas prêt ;
  - le rappel de DATA à l'état haut côté carte de puissance (≈ 5 kΩ vers VCC), que la carte de commande ne fait plus ;
  - deux connecteurs pour la nappe coupée (GND, VCC, DATA de chaque côté).
- [ ] Mesurer la place libre dans le boîtier de commande et choisir où loger la carte et comment couper la nappe (rallonge à connecteurs d'origine si possible).
- [ ] Firmware ESP32-C3 : réponse dans le bit qui suit l'appel (réception sur interruption ou UART matérielle), relais par défaut, état imposé par HA, retour au relais sur appui d'une touche ou sans HA pendant 30 min.
- [ ] Prototype au banc avec deux D1 mini ou un ESP32 à deux UART avant la carte : vérifier le relais complet, cartes branchées.
- [ ] Boîtier (`enclosure/`), séparation secteur / basse tension.

## À comprendre

- [ ] Capture longue avec une consigne proche de la température de la pièce : trouver la mesure de température dans `75`, `61` ou `73`, pour l'exposer à HA.
- [ ] Ce que fait la carte de puissance après un long silence de la carte de commande (au-delà de 30 s).
- [ ] Sens de la réponse `70` (`83 80 89`, constante) et de `76`.

## Côté Home Assistant

- [ ] Entités du MITM (`architecture.md`), puis thermostat HA dans le projet d'automatisation.
- [ ] Mesure de puissance éventuelle : Shelly PM Mini Gen3 dans la boîte de raccordement (≈ 1750 W = Boost, ≈ 750 W = fluide, 0 = attente), utile si `C7` ne suffit pas.

## Référence : carte de commande

Photographiée le 25/10/2026 ([dans son boîtier](photos/command-board-in-box.jpg), [touches et écran](photos/command-board-keys.jpg)) : étiquette `HM011 TD1N01 RAD`, circuit imprimé `74 60 5004 IndB`, microcontrôleur **STM32F091CCT6** (contrôleur tactile intégré), écran LCD Varitronix, touches capacitives (ressorts dorés vers la vitre), 5 pastilles alignées (probablement SWD, lecture du firmware sûrement verrouillée). La garantie (2 ans) est perdue dès qu'on ouvre le boîtier.
