"use strict";
exports.__esModule = true;
exports.createDefer = exports.sliently = exports.wait = exports.kTmp = void 0;
var path = require("path");
exports.kTmp = path.resolve(__dirname, './.tmp');
function wait(t) {
    return new Promise(function (resolve, reject) {
        setTimeout(function () {
            resolve();
        }, t);
    });
}
exports.wait = wait;
function sliently(fn) {
    try {
        fn();
    }
    catch (_) { }
}
exports.sliently = sliently;
function createDefer() {
    var resolve, reject;
    var p = new Promise(function (res, rej) {
        resolve = res;
        reject = rej;
    });
    return {
        p: p,
        resolve: resolve,
        reject: reject
    };
}
exports.createDefer = createDefer;
