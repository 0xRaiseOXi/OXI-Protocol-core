let tg = window.Telegram.WebApp;
let data_local;

tg.expand();
tg.setHeaderColor("#000000");

const elements = {
    claimTokensButton: document.getElementById("claim-tokens-button"),
    counterVault: document.getElementById('tokens-value-vault'),
    dataContainer: document.getElementById('main-balance'),
    referalsValue: document.getElementById('referals-value'),
    referalCodeContainer: document.getElementById('referal-code'),
    menuButton: document.getElementById("upgrade-menu-button"),
    overlay: document.getElementById("overlay"),
    inviteButton: document.getElementById("invite-friends-button"),
    closeMenuUpgrade: document.getElementById('close-menu-upgrade'),
    closeMenuUpgradeImg: document.getElementById('close-menu-upgrade-img'),
    confirmUpdateButton: document.querySelector('.confirm-update-button'),
    upgradeButtonModule1: document.getElementById('upgrade-button-module_1'),
    playerName: document.getElementById('player-name'),
    playerNameC: document.querySelector('.player-name-container'),
};

document.addEventListener("DOMContentLoaded", async () => {
    const user = tg.initDataUnsafe.user;
    const userData = { id: 1070221127 }; // временное значение для тестирования

    try {
        const dataUserFromServer = await sendDataToServer(userData);

        updateUserData(dataUserFromServer);
        loadUpgradesModule(dataUserFromServer);
    } catch (error) {
        console.error('Failed to fetch user data', error);
    }

    elements.claimTokensButton.addEventListener('click', async () => {
        const data = await claimTokens();
        animateMainCounter(data['oxi_tokens_value']);
        vaultProgressBar(0);
        elements.counterVault.textContent = 0;
        data_local = data;
    });

    elements.menuButton.addEventListener("click", () => {
        elements.overlay.style.display = "flex";
    });

    elements.overlay.addEventListener("click", (event) => {
        if (event.target === elements.overlay) {
            elements.overlay.style.display = "none";
        }
    });

    elements.closeMenuUpgrade.addEventListener('click', closeMenuUpgrade);
    elements.closeMenuUpgradeImg.addEventListener('click', closeMenuUpgrade);
    elements.confirmUpdateButton.addEventListener('click', () => update('miner', 'miner_1'));
    elements.upgradeButtonModule1.addEventListener('click', () => update('miner', 'miner_1'));
    
    setUserName();

    setInterval(vaultUpdate, 1000);
    loadText();
    vaultUpdate();
    tg.ready();
});

async function update(type, id) {
    const dataToSend = { _id: 1070221127, type_update: type, id_update: id };
    try {
        const response = await fetch('http://127.0.0.1:8080/api/update', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(dataToSend)
        });
        const result = await response.json();
        console.log(result);
        closeMenuUpgrade();
        loadUpgradesModule(result);
        data_local = result;
    } catch (error) {
        console.log("Error: ", error);
    }
}

async function sendDataToServer(dataToSend) {
    try {
        const response = await fetch('http://127.0.0.1:8080/api/data', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(dataToSend)
        });
        const result = await response.json();
        data_local = result;
        console.log(result);
        return result;
    } catch (error) {
        elements.dataContainer.textContent = 'Server Error';
        throw error;
    }
}

async function claimTokens() {
    try {
        const response = await fetch('http://127.0.0.1:8080/claim_tokens', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: 1070221127 })
        });
        if (!response.ok) {
            elements.claimTokensButton.textContent = "Server Error";
            return;
        }
        const data = await response.json();
        console.log(data);
        return data;
    } catch (error) {
        console.log(error);
        elements.claimTokensButton.textContent = "Server Error";
    }
}

function setUserName() {
    console.log("set name");
    const playerName = document.getElementById("player-name");
    if (data_local['first_name']) {
        if (data_local['last_name']) {
            playerName.textContent = data_local['first_name'] + " " + data_local['last_name'];   
        }
        playerName.textContent = data_local['first_name'];   
    }
    playerName.textContent = data_local['username']; 
}

function updateUserData(data) {
    elements.dataContainer.textContent = data['oxi_tokens_value'].toLocaleString('en-US');
    elements.referalCodeContainer.textContent = `https://t.me/oxi_protocol_bot?start=${data['referal_code']}`;
    elements.referalsValue.textContent = data['referals_value'];
}

function loadUpgradesModule(dataFromServer) {
    document.getElementById("miner_1_lvl").textContent = `lvl ${dataFromServer['upgrades_current']['miner_1']['level']}`;
    document.getElementById("miner_1_tokens_add").textContent = dataFromServer['upgrades_current']['miner_1']['tokens_hour'];
    document.getElementById("miner_1_price").textContent = dataFromServer['upgrades_new']['miner_1']['price'];
    document.getElementById("module_1-price-balance").textContent = dataFromServer['upgrades_new']['miner_1']['price'];
}

function closeMenuUpgrade() {
    elements.overlay.style.display = "none";
}

function copyText() {
    const textToCopy = elements.referalCodeContainer.innerText;
    const tempInput = document.createElement('textarea');
    tempInput.value = textToCopy;
    document.body.appendChild(tempInput);
    tempInput.select();
    document.execCommand('copy');
    document.body.removeChild(tempInput);
}

elements.inviteButton.addEventListener('click', async () => {
    const url = "Привет! Приглашаю тебя в новый проект OXI Mining Protocol. Скорее заходи " + elements.referalCodeContainer.textContent;
    const encodedUrl = encodeURIComponent(url);
    const telegramLink = `https://t.me/share/url?url=&text=${encodedUrl}`;
    tg.openTelegramLink(telegramLink);
});

function showSection(sectionId, element) {
    const sections = document.querySelectorAll('.content-section-show');
    sections.forEach(section => section.style.display = 'none');
    document.getElementById(sectionId).style.display = 'block';

    const buttons = document.querySelectorAll('.button-menu-buttons');
    buttons.forEach(button => button.classList.remove("active"));
    element.classList.add("active");
}

function vaultProgressBar(percentage) {
    const progressBar = document.getElementById('progress-bar');
    setProgress(Math.max(1, Math.min(percentage, 100)));

    function setProgress(percentage) {
        progressBar.style.height = percentage + '%';
    }
}

function vaultUpdate() {
    const currentTime = Math.floor(Date.now() / 1000);
    const timeDifference = currentTime - data_local['last_time_update'];
    const addedTokens = Math.trunc(timeDifference / 3600 * data_local['tokens_hour']);
    const vaultSize = data_local['upgrades_current']['vault_main']['volume'];
    const percentage = Math.trunc(addedTokens / vaultSize * 100);

    elements.counterVault.textContent = addedTokens < vaultSize ? Math.max(0, addedTokens) : vaultSize;
    vaultProgressBar(percentage);
}

function parseNumber(number) {
    return Number(number.replace(/,/g, ''));
}

function animateMainCounter(targetCount) {
    const counterElement = elements.dataContainer;

    function animateCounter(startValue, endValue, duration) {
        const startTime = performance.now();

        function updateCounter(currentTime) {
            const elapsedTime = currentTime - startTime;
            const progress = Math.min(elapsedTime / duration, 1);
            const currentCount = Math.floor(startValue + progress * (endValue - startValue));
            counterElement.textContent = currentCount.toLocaleString('en-US');
            if (progress < 1) {
                requestAnimationFrame(updateCounter);
            }
        }

        requestAnimationFrame(updateCounter);
    }

    animateCounter(parseNumber(counterElement.textContent), targetCount, 800);
}

function showA() {
    const confirmationMenu = document.getElementById('confirmationMenu');
    confirmationMenu.classList.remove('hidden1');
    confirmationMenu.classList.add('hidden');
}

function loadText() {
    const vault = document.getElementById('vault-text');
    const vaultText = document.getElementById('vault-text-main');
    fitTextToContainer(vault, vaultText);
    fitTextToContainer(elements.playerNameC, elements.playerName);
    fitTextToContainer(document.getElementById('referal-code-container'), document.querySelector('referal-code'));

}

function fitTextToContainer(container, textElement) {
    let fontSize = parseInt(window.getComputedStyle(textElement).getPropertyValue('font-size'));

    while ((textElement.scrollHeight > container.clientHeight || textElement.scrollWidth > container.clientWidth) && fontSize > 0) {
        fontSize--;
        textElement.style.fontSize = fontSize + 'px';
    }
}
