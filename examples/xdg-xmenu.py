#!/usr/bin/env python3
#
# Original version by OliverLew: https://github.com/OliverLew/xdg-xmenu
#

from pathlib import Path
import argparse
import os, os.path
import re
import shutil
import subprocess
import sys
import time

XDG_DATA_HOME       = os.getenv('XDG_DATA_HOME', os.path.join(os.getenv('HOME'), '.local/share'))
XDG_CONFIG_HOME     = os.getenv('XDG_CONFIG_HOME', os.path.join(os.getenv('HOME'), '.config'))
XDG_DATA_DIRS       = set(os.getenv('XDG_DATA_DIRS', '/usr/share:/usr/local/share').split(':'))
APPLICATION_DIRS    = set([os.path.join(dir, 'applications') for dir in XDG_DATA_DIRS.copy()]\
                        + [os.path.join(XDG_DATA_HOME, 'applications')])
CACHE_DIR           = os.path.join(os.getenv('XDG_CACHE_HOME',
                        os.path.join(os.getenv('HOME'), '.cache')), os.path.basename(__file__))
TERMINAL            = os.getenv('TERMINAL', 'xterm')
EXECFILTER          = re.compile('%[a-zA-Z]')
ICON_SIZE           = 24
IMAGEMAGICK_OPTIONS = ['-background', 'none', '-size', '{0}x{0}'.format(ICON_SIZE)]
MULTIPLE_CATEGORIES = True
PRINT_IMAGES        = True
SORT_BY_CATEGORY    = True
COMPILE_IMAGES      = True
FORCE_REFRESH_CACHE = False
EXPIRY_DAYS         = 7
EXPIRY_TIME         = EXPIRY_DAYS * 24 * 60 * 60
UPDATEEXPIRED       = False
USER_ICON_THEME     = ''

imagefiles = []
icontheme = None
applications = {}
categories = {}


class Category:
    label = ''
    iconname = ''
    iconpath = ''
    match = re.compile('a^')
    apps = {}

    def format(self):
        out = ''
        if PRINT_IMAGES and self.iconpath != '':
            out += 'IMG:{}\t'.format(self.iconpath)
        out += self.label
        return out


class Application:
    categories = set()
    execute = ''
    genname = ''
    iconname = ''
    iconpath = ''
    name = ''
    terminal = False
    nodisplay = False
    onlyshowin = ''

    def format(self):
        out = ''
        if PRINT_IMAGES and self.iconpath != '':
            out += 'IMG:{}\t'.format(self.iconpath)
        out += self.name
        if self.genname != '':
            out += ' ({})'.format(self.genname)
        if self.terminal:
            out += '\t{} -e {}'.format(TERMINAL, self.execute)
        else:
            out += '\t{}'.format(self.execute)
        return out


def add_category(label, iconname, matchstr):
    newcat = Category()
    newcat.label = label
    newcat.apps = {}
    newcat.iconname = iconname
    newcat.match = re.compile(matchstr)
    categories[label] = newcat


def cache_is_expired():
    now = time.time()
    if not os.path.isdir(CACHE_DIR):
        return False
    for filename in os.listdir(CACHE_DIR):
        filestamp = os.stat(os.path.join(CACHE_DIR, filename)).st_mtime
        filecompare = now - EXPIRY_TIME
        if filestamp >= filecompare:
            return False
    return True


def dict_to_application(dictionary:dict):
    app = Application()
    app.name = dictionary['Name']
    app.execute = re.sub(EXECFILTER, '', dictionary['Exec'])
    if 'GenericName' in dictionary:
        app.genname = dictionary['GenericName']
    if 'Icon' in dictionary and PRINT_IMAGES:
        app.iconname = os.path.basename(dictionary['Icon'])
        set_icon(app)
    if 'Categories' in dictionary:
        app.categories = dictionary['Categories'].split(';')
    if 'Terminal' in dictionary:
        app.terminal = dictionary['Terminal'] in ('true', 'True')
    if 'NoDisplay' in dictionary:
        app.nodisplay = dictionary['NoDisplay'] in ('true', 'True')
    if 'OnlyShowIn' in dictionary:
        app.onlyshowin = dictionary['OnlyShowIn'].split(';')

    if SORT_BY_CATEGORY:
        added = False
        for appcat in app.categories:
            if appcat in categories:
                if not app in categories[appcat].apps:
                    categories[appcat].apps[app.name] = app
                added = True
            else:
                for cat in categories.values():
                    if cat.match.match(appcat):
                        cat.apps[app.name] = app
                        added = True
                        break
            if added and not MULTIPLE_CATEGORIES:
                break
        if not added:
            categories['Other'].apps[app.name] = app

    return app


def get_image_files():
    filelist = []
    for icondir in [os.path.join(dd, 'icons') for dd in XDG_DATA_DIRS]:
        for root, dirs, files in os.walk(icondir):
            for f in files:
                file = os.path.join(root, f)
                if os.path.isfile(file):
                    filelist.append(file)
    return filelist


def get_icon_theme():
    if USER_ICON_THEME != '':
        return USER_ICON_THEME
    path = os.path.join(XDG_CONFIG_HOME, 'gtk-3.0/settings.ini')
    if os.path.isfile(path):
        file = open(path, 'r')
        for line in file.readlines():
            if 'gtk-icon-theme-name' in line:
                return line.strip().split('=')[1]
    return ''


def load_desktop_file(filepath):
    file = open(filepath, 'r')
    application_dict = {}
    for line in file.readlines():
        try:
            key, value = line.strip().split('=', 1)
            # cancel before additional options are added
            if key in application_dict:
                break
            application_dict[key] = value.strip()
        except: pass
    try:
        application = dict_to_application(application_dict)
        applications[application.name] = application
    except KeyError: pass


def load_desktop_files():
    for directory in APPLICATION_DIRS:
        if not os.path.isdir(directory):
            continue
        for subfile in os.listdir(directory):
            filepath = os.path.join(directory, subfile)
            if os.path.islink(filepath):
                filepath = os.readlink(filepath)
            if os.path.isfile(filepath):
                load_desktop_file(filepath)


def load_icon(name, ext, path):
    if not os.path.isdir(CACHE_DIR):
        os.makedirs(CACHE_DIR)
    dest = os.path.join(CACHE_DIR, name + '.png')
    if ext in ('png'):
        shutil.copyfile(path, dest)
    else:
        proc = subprocess.Popen(['convert'] + IMAGEMAGICK_OPTIONS + [path, dest], stdout=subprocess.PIPE)
        proc.wait()
    if os.path.isfile(dest):
        return dest
    else:
        return ''


def set_icon(entity):
    global imagefiles, icontheme
    name = entity.iconname
    if not FORCE_REFRESH_CACHE and (os.path.isfile(os.path.join(CACHE_DIR, name + '.png'))):
        entity.iconpath = os.path.join(CACHE_DIR, name + '.png')
    elif not FORCE_REFRESH_CACHE and (os.path.isfile(os.path.join(CACHE_DIR, name + '.notfound'))):
        pass
    elif COMPILE_IMAGES or FORCE_REFRESH_CACHE:
        image_found = False
        if imagefiles == []:
            imagefiles = get_image_files()
        if icontheme == None:
            icontheme = get_icon_theme()
        for imagefile in imagefiles:
            for ext in ['png', 'svg']:
                if imagefile.endswith('/{}.{}'.format(name, ext)) \
                        and (entity.iconpath == '' or (type(icontheme) == str and icontheme in imagefile)):
                    entity.iconpath = load_icon(name, ext, imagefile)
                    print("{}.png <= {}".format(name, imagefile), file=sys.stderr)
                    image_found = True
        if not image_found:
            if not os.path.exists(CACHE_DIR):
                os.makedirs(CACHE_DIR)
            Path(os.path.join(CACHE_DIR, name + '.notfound')).touch()
            print("=> {}.notfound".format(name), file=sys.stderr)


def format_xmenu(applications):
    if SORT_BY_CATEGORY:
        if PRINT_IMAGES:
            for cat in categories.values():
                set_icon(cat)
        for cat in categories.values():
            visible_apps = [app for app in cat.apps.values() if not app.nodisplay and app.onlyshowin == '']
            if len(visible_apps) > 0:
                print(cat.format())
                for app in sorted(visible_apps, key=lambda x: x.name):
                    print('\t{}'.format(app.format()))
    else:
        for app in sorted(applications, key=lambda x: x.name):
            print(app.format())


def parse_args():
    parser = argparse.ArgumentParser(description='Generate application menu in xmenu format')
    parser.add_argument('-a', '--applications', help='don\'t sort applications by category',
                        action='store_false', dest='sort_by_category')
    parser.add_argument('-e', '--expired', help='refresh image cache if older than {} days'.format(EXPIRY_DAYS),
                        action='store_true', dest='updateexpired')
    parser.add_argument('-f', '--force', help='force refresh image cache',
                        action='store_true', dest='force_refresh_cache')
    parser.add_argument('-i', '--icontheme', help='select a custom icon theme', nargs=1, type=str,
                        default=[''],  metavar='THEME', action='store', dest='user_icon_theme')
    parser.add_argument('-l', '--lazy', help='don\'t compile images on demand (this is way faster)',
                        action='store_false', dest='compile_images')
    parser.add_argument('-m', '--multiple', help='add applications to multiple categories',
                        action='store_true', dest='multiple_categories')
    parser.add_argument('-t', '--text', help='no image output (implies -l)',
                        action='store_false', dest='print_images')
    return parser.parse_args()


add_category('Multimedia',    'applications-multimedia',    '(Audio|Video).*')
add_category('Development',   'applications-development',   'Development')
add_category('Education',     'applications-education',     'Education')
add_category('Games',         'applications-games',         'Game')
add_category('Graphics',      'applications-graphics',      'Graphics')
add_category('Internet',      'applications-internet',      'Network')
add_category('Office',        'applications-office',        'Office')
add_category('Science',       'applications-science',       'Science')
add_category('Settings',      'preferences-desktop',        'Settings')
add_category('System',        'applications-system',        'System')
add_category('Accessories',   'applications-accessories',   'Utility')
add_category('Others',        'applications-other',         'a^')


if __name__ == '__main__':
    args = parse_args()
    MULTIPLE_CATEGORIES = args.multiple_categories
    SORT_BY_CATEGORY = args.sort_by_category
    PRINT_IMAGES = args.print_images
    COMPILE_IMAGES = args.compile_images and PRINT_IMAGES
    UPDATEEXPIRED = args.updateexpired
    FORCE_REFRESH_CACHE = args.force_refresh_cache or (UPDATEEXPIRED and cache_is_expired())
    USER_ICON_THEME = args.user_icon_theme[0]
    load_desktop_files()
    format_xmenu(list(applications.values()))
