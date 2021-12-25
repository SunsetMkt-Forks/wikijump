<?php

namespace Wikidot\DB;




use Ozone\Framework\Database\BaseDBPeer;

/**
 * Base peer Class mapped to the database table comment_revision.
 */
class CommentRevisionPeerBase extends BaseDBPeer
{
    public static $peerInstance;

    protected function internalInit()
    {
        $this->tableName='comment_revision';
        $this->objectName=CommentRevision::class;
        $this->primaryKeyName = 'revision_id';
        $this->fieldNames = array( 'revision_id' ,  'comment_id' ,  'user_id' ,  'user_string' ,  'text' ,  'title' ,  'date' );
        $this->fieldTypes = array( 'revision_id' => 'serial',  'comment_id' => 'int',  'user_id' => 'int',  'user_string' => 'varchar(80)',  'text' => 'text',  'title' => 'varchar(256)',  'date' => 'timestamp');
        $this->defaultValues = array();
    }

    public static function instance()
    {
        if (self::$peerInstance == null) {
            $className = CommentRevisionPeer::class;
            self::$peerInstance = new $className();
        }
        return self::$peerInstance;
    }
}
