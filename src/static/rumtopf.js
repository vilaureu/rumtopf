"use strict";

function round(n) {
  return parseFloat(n.toFixed(2));
}

function calc_recipe() {
  const inputs = document.getElementsByClassName("servings_input");
  const scalings = document.getElementsByClassName("scaling");
  const submits = document.getElementsByClassName("servings_submit");

  for (const elem of submits) {
    if (elem instanceof HTMLInputElement) elem.disabled = false;
  }

  const params = new URLSearchParams(document.location.search);
  let servings = parseFloat(params.get("servings"));
  if (Number.isNaN(servings)) servings = parseFloat(inputs[0]?.value);
  if (Number.isNaN(servings)) servings = 1;

  let base = parseFloat(inputs[0]?.dataset["base"]);
  if (Number.isNaN(base)) base = 1;
  let factor = servings / base;

  for (const elem of scalings) {
    let base = parseFloat(elem.dataset["base"]);
    if (Number.isNaN(base)) continue;
    elem.textContent = round(factor * base).toLocaleString(
      document.documentElement.lang
    );
  }
  for (const elem of inputs) {
    if (elem instanceof HTMLInputElement) elem.value = servings;
  }
}

window.addEventListener("DOMContentLoaded", (_) => calc_recipe());
