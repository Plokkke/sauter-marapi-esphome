# Backlog

## Fait

- [x] Bus Linap décodé : format, dialogue, consigne, Boost, arrêt (`linap.md`).
- [x] La carte de puissance obéit à un ESP qui joue la carte de commande (consigne, Boost, arrêt).
- [x] La carte de commande répond à un ESP qui joue la carte de puissance : tout ce qui est fait aux touches est lisible.
- [x] La carte de commande ne se laisse pas piloter par le bus (adresses et écritures essayées) : l'écran ne suivra pas HA.
- [x] Crate `linap` en Rust pur, testée sur le Mac : trames, parité, octet d'appel, mode transparent, état imposé, retour au transparent sur changement aux touches ou sans HA pendant 30 min (`firmware/linap/`).
- [x] Firmware du C3 écrit, compile : deux UART sur interruption, Wi-Fi, MQTT et découverte HA (`firmware/mitm/`).
- [x] Capacité du 5 V de la nappe : ≈ 160 mA en tout, lâche vers 180 mA. Un D1 mini y tient avec le boîtier de commande (démarrages et Boost validés) : plus de 230 V à raccorder.

## Carte MITM

- [ ] Dessiner la carte MITM (`hardware/`, règles de dessin dans `CLAUDE.md`) :
  - sur un ESP32-C3 à antenne externe, alimenté par la nappe, avec un condensateur de 470 à 1000 µF ;
  - deux voies, côté commande et côté puissance, chacune avec le montage du banc (pont diviseur en lecture, NPN en émission) ;
  - le rappel de DATA à l'état haut côté carte de puissance (≈ 5 kΩ vers VCC), que la carte de commande ne fait plus ;
  - deux connecteurs pour la nappe coupée (GND, VCC, DATA de chaque côté).
- [ ] Mesurer la place libre dans le boîtier de commande et choisir où loger la carte et comment couper la nappe (rallonge à connecteurs d'origine si possible).
- [ ] Choisir la carte C3 (antenne externe) et mesurer sa consommation sur la nappe, Wi-Fi actif, démarrages du radiateur et Boost compris.
- [ ] Mesurer la marge de réponse de la carte de puissance (étape 3, réglage « Reply gap » à 2, 5, 10, 20 bits, Boost à chaque palier) : elle fixe la contrainte de temps du firmware.

## Firmware Rust (`firmware/`)

- [ ] Compte MQTT pour la carte sur le Mosquitto de HA, dans `firmware/secrets.env`.
- [ ] Premier essai du C3 sur le radiateur, carte de commande débranchée (comme l'étape 3) : `Control` sur `setpoint` et `boost` depuis HA, « Polls answered » à 100 %, mesure de la marge « Reply gap » et de la consommation sur la nappe.
- [ ] Puis le MITM complet, carte de commande sur la seconde voie : relais des touches, reprise de la main, écran.
- [ ] Mises à jour OTA (`esp-bootloader-esp-idf`, deux partitions), indispensables une fois la carte enfermée.
- [ ] Boîtier (`enclosure/`), séparation secteur / basse tension.

## À comprendre

- [ ] Capture longue avec une consigne proche de la température de la pièce : trouver la mesure de température dans `75`, `61` ou `73`, pour l'exposer à HA. Avec le journal des trames du MITM (« Frame log », `firmware/README.md`), plus besoin du sniffer.
- [ ] Ce que fait la carte de puissance après un long silence de la carte de commande (au-delà de 30 s).
- [ ] Sens de la réponse `70` (`83 80 89`, constante) et de `76`.

## Côté Home Assistant

- [ ] Entités du MITM (`architecture.md`), puis thermostat HA dans le projet d'automatisation.
- [ ] Mesure de puissance éventuelle : Shelly PM Mini Gen3 dans la boîte de raccordement (≈ 1750 W = Boost, ≈ 750 W = fluide, 0 = attente), utile si `C7` ne suffit pas.

## Référence : carte de commande

Photographiée le 25/10/2026 ([dans son boîtier](photos/command-board-in-box.jpg), [touches et écran](photos/command-board-keys.jpg)) : étiquette `HM011 TD1N01 RAD`, circuit imprimé `74 60 5004 IndB`, microcontrôleur **STM32F091CCT6** (contrôleur tactile intégré), écran LCD Varitronix, touches capacitives (ressorts dorés vers la vitre), 5 pastilles alignées (probablement SWD, lecture du firmware sûrement verrouillée). La garantie (2 ans) est perdue dès qu'on ouvre le boîtier.
