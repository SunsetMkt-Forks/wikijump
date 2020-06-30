/*
 * Wikidot - free wiki collaboration software
 * Copyright (c) 2008-2020, Wikidot Inc., SCP Wiki Technical Team
 *
 * Code licensed under the GNU Affero General Public
 * License version 3 or later.
 *
 * For more information about licensing visit:
 * http://www.wikidot.org/license
 */

WIKIDOT.modules.PMSentModule = {};

WIKIDOT.modules.PMSentModule.vars = {
	currentMessageId: null
}

WIKIDOT.modules.PMSentModule.listeners = {
	loadList: function(e, pageNo){
		var p = null;
		if(pageNo){p = {page: pageNo}};
		OZONE.ajax.requestModule("account/pm/PMSentModule", p, WIKIDOT.modules.AccountMessagesModule.callbacks.setActionArea);
		if(e){	WIKIDOT.modules.AccountMessagesModule.utils.highlightTab(e);}
	},

	selectAll: function(e){
		var chs = YAHOO.util.Dom.getElementsByClassName("message-select");
		for(var i=0; i<chs.length; i++){
			chs[i].checked=true;
		}

	},

	removeSelected: function(e){
		var selected = new Array();
		var chs = YAHOO.util.Dom.getElementsByClassName("message-select");
		for(var i=0; i<chs.length; i++){
			if(chs[i].checked){
				selected.push(chs[i].id.replace(/message\-check\-/, ''));
			}
		}
		if(selected.length == 0){
			return;
		}
		var p = new Object();
		p.action = "PMAction";
		p.event = 'removeSelectedSent';
		p.selected = JSON.stringify(selected);
		OZONE.ajax.requestModule(null, p, WIKIDOT.modules.PMSentModule.callbacks.removeSelected);

	},
	removeSentMessage: function(e, messageId){
		WIKIDOT.modules.PMSentModule.vars.currentMessageId = messageId;
		var w = new OZONE.dialogs.ConfirmationDialog();
		w.content = "Are sure you want to remove this message?";
		w.buttons = ['cancel', 'remove message'];
		w.addButtonListener('cancel', w.close);
		w.addButtonListener('remove message', WIKIDOT.modules.PMSentModule.listeners.removeSentMessage2);
		w.focusButton = 'cancel';
		w.show();
	},

	removeSentMessage2: function(e, messageId){
		var p = new Object();
		p.action = "PMAction";
		p.event = 'removeSentMessage';
		p.message_id = WIKIDOT.modules.PMSentModule.vars.currentMessageId;
		OZONE.ajax.requestModule(null, p, WIKIDOT.modules.PMSentModule.callbacks.removeSentMessage);
	}
}

WIKIDOT.modules.PMSentModule.callbacks = {
	removeSelected: function(r){
		WIKIDOT.modules.PMSentModule.listeners.loadList(null, 1);
	},
	removeSentMessage: function(r){
		if(r.status == 'ok'){
			var w = new OZONE.dialogs.SuccessBox();
			w.content = "The message has been removed.";
			w.show();

			if(r.messageId){
				setTimeout('WIKIDOT.modules.AccountMessagesModule.listeners.viewSentMessage("'+r.messageId+'")', 1000);
			}else{
				// return to inbox view
				setTimeout('sentPage(1)');
			}
		}
	}
}

WIKIDOT.modules.PMSentModule.init = function(){

}
