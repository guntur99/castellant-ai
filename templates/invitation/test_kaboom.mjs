import fs from 'fs';
import { JSDOM } from 'jsdom';
const dom = new JSDOM(`<!DOCTYPE html><div id="game-container"></div>`);
global.window = dom.window;
global.document = dom.window.document;
global.navigator = dom.window.navigator;
global.HTMLElement = dom.window.HTMLElement;
global.HTMLCanvasElement = dom.window.HTMLCanvasElement;
global.requestAnimationFrame = (cb) => setTimeout(cb, 16);

import kaboom from "./kaboom.mjs";

const k = kaboom({ root: document.getElementById("game-container"), global: false });
try {
    k.scale(1);
    console.log("k.scale(1) works");
} catch(e) {
    console.log("k.scale(1) error:", e.message);
}

try {
    const player = k.add([ k.pos(0,0), k.scale(1) ]);
    player.scale = k.vec2(2, 2);
    console.log("player.scale works");
} catch(e) {
    console.log("player.scale error:", e.message);
}
