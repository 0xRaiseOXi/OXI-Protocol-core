let tg = window.Telegram.WebApp;
let data_local;

tg.expand();
tg.setHeaderColor("#000000");

const elements = {
    counterVault: document.getElementById('tokens-value-vault'),
    dataContainer: document.getElementById('main-balance'),

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
    mineHour: document.getElementById('mine-in-hour'),
    overlayFabric: document.getElementById('overlay-fabric'),
    overlayDrone: document.getElementById('overlay-drone'),
    overlayAwards: document.getElementById('overlay-awards'),
    claimTokensButton: document.getElementById("claim-tokens-button"),
};

document.addEventListener("DOMContentLoaded", async () => {
    const userData = { id: tg.initDataUnsafe.user.id }; 
    console.log(userData);
    const dataUserFromServer = await sendDataToServer(userData);
    setStartData(dataUserFromServer);

    elements.claimTokensButton.addEventListener('click', async () => {
        if (elements.claimTokensButton.classList.contains("deactive")) {
            return;
        }
        await claimTokens();
        notification("Токены OXI собраны");
    });


    elements.overlayFabric.addEventListener("click", (event) => {
        if (event.target === elements.overlayFabric) {
            elements.overlayFabric.style.display = "none";
        }
    });

    document.querySelector(".module-1-2").addEventListener("click", () => {
        elements.overlayFabric.style.display = "flex";
    });

    document.getElementById("close-menu-buy-fabric").addEventListener("click", () => {
        elements.overlayFabric.style.display = "none";
    });


    document.querySelector(".module-1-1").addEventListener("click", () => {
        elements.overlayDrone.style.display = "flex";
    });

    document.getElementById("close-menu-buy-drone").addEventListener("click", () => {
        elements.overlayDrone.style.display = "none";
    });

    elements.overlayDrone.addEventListener("click", (event) => {
        if (event.target === elements.overlayDrone) {
            elements.overlayDrone.style.display = "none";
        }
    });


    document.querySelector(".module-1-3").addEventListener("click", () => {
        elements.overlayAwards.style.display = "flex";
    });

    document.getElementById("close-menu-buy-awards").addEventListener("click", () => {
        elements.overlayAwards.style.display = "none";
    });

    elements.overlayAwards.addEventListener("click", (event) => {
        if (event.target === elements.overlayAwards) {
            elements.overlayAwards.style.display = "none";
        }
    });

    const miners = document.querySelectorAll(".data-upgarde-module-lock");
    miners.forEach(miner => {
        let minerClass = '';
        miner.classList.forEach(cls => {
            if (cls.startsWith('miner_')) {
                minerClass = cls;
            }
        });

        document.getElementById(minerClass + "_price-buy").textContent = 1000*3;

        const overlay = document.getElementById(minerClass + "_overlay");
        overlay.addEventListener("click", (event) => {
            if (event.target === overlay) {
                overlay.style.display = "none";
            }
        });

        document.getElementById(minerClass + "_unlock-module").addEventListener('click', async () => {
            overlay.style.display = "flex";
        });


        document.getElementById(minerClass + "_confirm-update-button").addEventListener('click', async () => {
            await update("miner", minerClass);
            overlay.style.display = "none";
            
            document.getElementById(minerClass + "_lvl").textContent = "lvl " + data_local['upgrades_current'][minerClass]['level'];
            document.getElementById(minerClass + "_tokens_add").textContent = data_local['upgrades_current'][minerClass]['tokens_hour'];
            document.getElementById(minerClass + "_price").textContent = data_local['upgrades_new'][minerClass]['price'];
            document.getElementById(minerClass + "_price-buy").textContent = data_local['upgrades_new'][minerClass]['price'];

            document.getElementById(minerClass + "_lock").style.display = "none";

            document.getElementById(minerClass + "_upgrade_button").addEventListener('click', () => {
                overlay.style.display = "flex";
            });
        });

        if (minerClass in data_local['upgrades_current']) {
            document.getElementById(minerClass + "_lock").style.display = "none";

            document.getElementById(minerClass + "_lvl").textContent = "lvl " + data_local['upgrades_current'][minerClass]['level'];
            document.getElementById(minerClass + "_tokens_add").textContent = data_local['upgrades_current'][minerClass]['tokens_hour'];
            document.getElementById(minerClass + "_price").textContent = data_local['upgrades_new'][minerClass]['price'];
            document.getElementById(minerClass + "_price-buy").textContent = data_local['upgrades_new'][minerClass]['price'];

            document.getElementById(minerClass + "_upgrade_button").addEventListener('click', () => {
                overlay.style.display = "flex";
            });

            // console.log(minerClass + "_close_overlay");
            // document.getElementById(minerClass + "_close_overlay").addEventListener('click', () => {
            //     overlay.style.display = "none";
            // });

        }
    });

    set_timer();
    progeressXpLevel();

    setInterval(vaultUpdate, 1000);
    loadText();
    vaultUpdate();
    tg.ready();
});

async function update(type, id) {
    const dataToSend = { _id: tg.initDataUnsafe.user.id, type_update: type, id_update: id };
    if (data_local['oxi_tokens_value'] < document.getElementById(id + "_price-buy").textContent) {
        notification("Недостаточный баланс");
        return;
    }
    try {
            const response = await fetch('https://oxiprotocol.ru/api/update', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(dataToSend)
        });
        const result = await response.json();
        data_local = result;
        console.log("up");
        console.log(result);
        await claimTokens();

        updateMainView(result);
        notification("Успешно");
    } catch (error) {
        console.log("Error: ", error);
    }
}

async function sendDataToServer(dataToSend) {
    try {
            const response = await fetch('https://oxiprotocol.ru/api/data', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(dataToSend)
        });
        const result = await response.json();
        data_local = result;
        console.log("get");
        console.log(result);
        return result;
    } catch (error) {
        console.log("Error: ", error);
    }
}

async function claimTokens() {
    try {
            const response = await fetch('https://oxiprotocol.ru/claim_tokens', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: tg.initDataUnsafe.user.id })
        });
        if (!response.ok) {
            return;
        }
        const data = await response.json();
        data_local = data;
        console.log("cl");
        console.log(data);
        animateMainCounter(data['oxi_tokens_value']);
        vaultProgressBar(0);
        progeressXpLevel();
        reload_timer();
        updateWar(data['war']);
        elements.counterVault.textContent = 0;
        elements.claimTokensButton.classList.add("deactive");
    } catch (error) {
        console.log(error);
    }
}

function setStartData(dataFromServer) {
    elements.dataContainer.textContent = dataFromServer['oxi_tokens_value'].toLocaleString('en-US');

    document.getElementById('referal-code').textContent = `https://t.me/oxi_protocol_bot?start=${dataFromServer['referal_code']}`;
    document.getElementById("referals-value").textContent = dataFromServer['referals_value'];
    document.getElementById('friends-add-value').textContent = formatNumber(dataFromServer['referals_value'] * 25000);

    const playerName = document.getElementById("player-name");

    const username = tg.initDataUnsafe.user.username;
    const first_name = tg.initDataUnsafe.user.first_name;
    const last_name = tg.initDataUnsafe.user.last_name;

    if (username) {
        playerName.textContent = username; 
    } else {
        if (first_name) {
            if (last_name) {
                playerName.textContent = first_name + " " + last_name;  
            } 
        }
        playerName.textContent = first_name;   
    }
    
    elements.mineHour.textContent = "+" + formatNumber(dataFromServer['tokens_hour']);

    updateWar(dataFromServer['war']);
}

// function updateViewUpgrade() {
//     const locks = document.querySelectorAll(".lock-text-sloi");
//     locks.forEach(lock => {
//         const classes = lock.classList;
//         let moduleClass = '';
//         classes.forEach(cls => {
//             if (cls.startsWith('miner_')) {
//                 moduleClass = cls;
//             }
//         });
//         if (moduleClass in data_local['upgrades_current']) {
//             lock.style.display = "none";
//             updateViewModule(moduleClass);
//             setEventOverlay(moduleClass);

//         }
//     });
//     elements.mineHour.textContent = "+" + formatNumber(data_local['tokens_hour']);
// }

// function updateViewModule(module) {
//     const typeModule = module.split("_")[1];
//     document.getElementById("miner_"+typeModule+"_lvl").textContent = `lvl ${data_local['upgrades_current'][module]['level']}`;
//     document.getElementById("miner_"+typeModule+"_tokens_add").textContent = data_local['upgrades_current'][module]['tokens_hour'];
//     document.getElementById("miner_"+typeModule+"_price").textContent = data_local['upgrades_new'][module]['price'];
//     document.getElementById("module_"+typeModule+"-price-balance").textContent = data_local['upgrades_new'][module]['price'];
// }

function setEventOverlay(module) {
    const typeModule = module.split("_")[1];

    document.getElementById("upgrade-menu-button_" + typeModule).addEventListener("click", () => {
        console.log("click" + " overlay" + typeModule);
        document.getElementById("overlay" + typeModule).style.display = "flex";
    });

    document.getElementById("overlay" + typeModule).addEventListener("click", (event) => {
        if (event.target === document.getElementById("overlay" + typeModule)) {
            document.getElementById("overlay" + typeModule).style.display = "none";
        }
    });

    document.getElementById("confirm-update-button_module_" + typeModule).addEventListener("click", (event) => {
        update("miner", module);
    });
}

function updateWar(level) {
    if (level < 10000) {
        document.getElementById('war').textContent = level; 
    } else {
        document.getElementById('war').textContent = formatNumber(level); 
    }
}

function updateMainView(data) {
    elements.mineHour.textContent = "+" + formatNumber(data['tokens_hour']);
    elements.dataContainer.textContent = data['oxi_tokens_value'].toLocaleString('en-US');
    if (data['war'] < 10000) {
        document.getElementById('war').textContent = data['war']; 
    } else {
        document.getElementById('war').textContent = formatNumber(data['war']); 
    }
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
    notification("Скопировано");
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
    const vaultSize = data_local['tokens_hour'] * 8;
    
    if (addedTokens > vaultSize) {
        elements.counterVault.textContent = vaultSize.toLocaleString('en-US');
    } else {
        elements.counterVault.textContent = Math.max(0, addedTokens).toLocaleString('en-US');
    }
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
}

function fitTextToContainer(container, textElement) {
    let fontSize = parseInt(window.getComputedStyle(textElement).getPropertyValue('font-size'));

    while ((textElement.scrollHeight > container.clientHeight || textElement.scrollWidth > container.clientWidth) && fontSize > 0) {
        fontSize--;
        textElement.style.fontSize = fontSize + 'px';
    }
}

let timer;

function reload_timer() {
    console.log('r');
    clearInterval(timer);
    set_timer();
}

function set_timer() {
    function formatTime(time) {
        let hours = Math.floor(time / 3600);
        let minutes = Math.floor((time % 3600) / 60);
        let seconds = Math.floor(time % 60);

        if (hours < 10) hours = '0' + hours;
        if (minutes < 10) minutes = '0' + minutes;
        if (seconds < 10) seconds = '0' + seconds;

        return `${hours}h ${minutes}m`;
    }
    function startTimer(duration, display) {
        let start = Date.now();
        let end = start + duration * 1000;

        function countdown() {
            let now = Date.now();
            let remaining = Math.floor((end - now) / 1000);
            let elapsed = (8 * 60 * 60) - remaining;
            let percentElapsed = (elapsed / (8 * 60 * 60)) * 100;
            if (percentElapsed > 80) {
                if (document.getElementById("claim-tokens-button").classList.contains("deactive")) {
                    document.getElementById("claim-tokens-button").classList.remove("deactive");
                }
            }
            vaultProgressBar(percentElapsed);

            display.textContent = formatTime(remaining);
            if (remaining <= 0) {
                clearInterval(timer);
                display.textContent = "00:00:00";
            }
        }

        countdown();
        timer = setInterval(countdown, 1000);
    }

    let lastUpdateTime = data_local['last_time_update'];
    let duration = 8 * 60 * 60;
    let now = Math.floor(Date.now() / 1000);
    let elapsed = now - lastUpdateTime;

    let display = document.getElementById('timer');

    if (elapsed < duration) {
        let remainingDuration = duration - elapsed;
        startTimer(remainingDuration, display);
    } else {
        display.textContent = "Full";
        if (document.getElementById("claim-tokens-button").classList.contains("deactive")) {
            document.getElementById("claim-tokens-button").classList.remove("deactive");
        }
        vaultProgressBar(100);
    }
}


function notification(text) {
    var notification = document.getElementById('notification');
    notification.classList.add('show');
    notification.textContent = text;
    setTimeout(function() {
        notification.classList.remove('show');
    }, 3000);
};


function formatNumber(num) {
    if (num >= 1_000_000) {
        if (num % 1_000_000 == 0) {
            return (num / 1_000_000) + 'M';
        } else {
            return (num / 1_000_000).toFixed(1) + 'M';
        }
    } else if (num >= 1_000) {
        if (num % 1_000== 0) {
            return (num / 1_000 ) + 'k';
        } else {
            return (num / 1_000).toFixed(1) + 'k';
        }
    } else {
        return num.toString();
    }
}

function progeressXpLevel() {
    const totalLevels = 60;
    const increasePercentage = 1.2; 
    const initialExp = 100;

    const levels = [initialExp];

    for (let i = 1; i < totalLevels; i++) {
        const previousLevelExp = levels[i - 1];
        const newLevelExp = Math.floor(previousLevelExp * increasePercentage);
        levels.push(newLevelExp);
    }

    let currentLevel = 0;
    let nextLevelXP = levels[0];
    let currentXP = data_local['level'];

    for (let i = 0; i < levels.length; i++) {
        if (currentXP < levels[i]) {
            nextLevelXP = levels[i];
            break;
        }
        currentLevel = i + 1;
    }

    const previousLevelXP = currentLevel > 0 ? levels[currentLevel - 1] : 0;
    const xpInCurrentLevel = currentXP - previousLevelXP;
    const xpNeededForNextLevel = nextLevelXP - previousLevelXP;
    const progressPercentage = (xpInCurrentLevel / xpNeededForNextLevel) * 100;
    document.querySelector('.player-progress-bar').style.width = progressPercentage + '%';
    document.querySelector('.player-level').textContent = currentLevel;
}