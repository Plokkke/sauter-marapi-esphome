# Backlog

- [ ] Vérifier sur l'appareil que le mode **Programme Inactif** est sélectionné.
- [ ] Mesurer la place disponible dans la boîte de raccordement ; sinon boîtier IP44 séparé, hors volumes 0 et 1.
- [ ] Prototype sur plaque, test à vide : mesure à l'oscilloscope ou au multimètre (AC/DC) entre fil pilote et neutre pour chaque ordre.
- [ ] Boîtier imprimé en 3D (`enclosure/`), séparation 230 V / basse tension.
- [ ] **Boost (soufflerie)** : aucun ordre fil pilote ne le déclenche. Carte du boîtier de commande photographiée le 25/10/2026 :
  - étiquette `HM011 TD1N01 RAD`, circuit imprimé `74 60 5004 IndB` ; microcontrôleur **STM32F091CCT6** (Cortex-M0, LQFP48, contrôleur tactile capacitif TSC intégré), écran LCD Varitronix sur nappe, buzzer CMS ;
  - **touches capacitives** : 5 ressorts dorés soudés sur des électrodes, qui prolongent l'électrode jusqu'à la vitre ; aucun contact mécanique à court-circuiter ;
  - liaison vers la carte de puissance : câble 3 fils (noir, blanc, rouge), probablement masse, alimentation et données (Linap) ;
  - 5 pastilles alignées (recto et verso) : probablement le port de programmation SWD. La lecture du firmware est sûrement verrouillée (RDP), donc pas une piste prioritaire.
- [ ] Piste retenue pour le Boost : simuler un doigt sur l'électrode de la touche Boost. Un photoMOS isolé (ex. AQY212) en série avec un condensateur de 22 à 100 pF, à ajuster au banc, relie la pastille du ressort à la masse de la carte. Le TSC recalibre sa ligne de base au démarrage, ce qui absorbe la capacité du photoMOS ouvert. Masse de la carte probablement au potentiel du secteur : mesurer avant tout branchement, isolement obligatoire, et la garantie (2 ans) saute. Pas de retour d'état : la durée du boost est fixée dans le Menu Expert (10 à 60 min), HA la suit avec une minuterie ;
  - à défaut : Confort pendant 30 min avec une consigne haute (750 W de fluide seulement, montée lente).
- [ ] **Mesure de puissance** : Shelly PM Mini Gen3 (sans relais) dans la boîte de raccordement, ou PZEM-004T v3 sur l'UART du module (`pzemac`). Sert aussi de retour d'état : ≈ 1750 W = Boost, ≈ 750 W = fluide, 0 = en attente.
- [ ] **Rétro-ingénierie de la liaison Linap** (boîtier de commande ↔ carte de puissance, cf. « Err 10 ») : photos des deux cartes, analyseur logique + PulseView, **isolateur USB ADuM3160 obligatoire** (carte non isolée du secteur). Objectif : lire consigne, mode, boost, température ; au mieux, les écrire.
- [ ] Côté HA : entité `climate` (sonde de pièce + thermostat HA qui bascule Confort / Éco), dans le projet d'automatisation.
